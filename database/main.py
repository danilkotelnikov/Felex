#!/usr/bin/env python3
"""Main controller for the feed database parser."""
from __future__ import annotations

import argparse
import asyncio
import logging
from pathlib import Path

import aiohttp

from config import DEFAULT_HEADERS, ERRORS_LOG_FILE, MAX_WORKERS, OUTPUT_DIR, REQUEST_DELAY
from discovery import discover_all_urls, fetch_page
from models import FeedItem, TranslatableText
from output import OutputGenerator
from parser import parse_feed_card
from progress import ProgressDisplay, ProgressManager
from translator import Translator


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler(ERRORS_LOG_FILE, encoding="utf-8"),
        logging.StreamHandler(),
    ],
)
logger = logging.getLogger(__name__)


class FeedParser:
    """Main parser controller."""

    def __init__(
        self,
        workers: int = MAX_WORKERS,
        resume: bool = False,
        discover_only: bool = False,
        download_only: bool = False,
        retry_failed: bool = False,
        output_dir: Path = OUTPUT_DIR,
        verbose: bool = False,
        validate: bool = True,
        translate_api: bool = False,
    ):
        self.workers = workers
        self.resume = resume
        self.discover_only = discover_only
        self.download_only = download_only
        self.retry_failed = retry_failed
        self.output_dir = Path(output_dir)
        self.verbose = verbose
        self.validate = validate
        self.translate_api = translate_api

        self.progress = ProgressManager(reset=not resume)
        self.display = ProgressDisplay()
        self.translator = Translator(use_api=translate_api)
        self.output = OutputGenerator(self.output_dir)
        if resume:
            self.output.load_existing_categories()

        self._semaphore = asyncio.Semaphore(workers)
        self._io_lock = asyncio.Lock()
        self._processed = 0
        self._errors = 0

    async def run(self) -> None:
        """Run the parsing pipeline."""

        try:
            if not self.download_only:
                await self._discovery_phase()
                if self.discover_only:
                    return
            elif self.progress.state.total_discovered == 0:
                raise RuntimeError("Download-only mode requires an existing progress.json with discovered URLs")

            await self._download_phase()
            self._generate_output()
            self.display.print_summary(self.progress.get_stats())
        except KeyboardInterrupt:
            logger.info("Interrupted, saving progress...")
            self.progress.save()
            raise
        except Exception:
            self.progress.save()
            raise
        finally:
            self.display.finish()

    async def _discovery_phase(self) -> None:
        if self.resume and self.progress.state.phase == "download" and self.progress.state.total_discovered > 0:
            logger.info("Skipping discovery because resume data already exists")
            return

        logger.info("Starting URL discovery...")
        await discover_all_urls(self.progress, self.display)
        logger.info("Discovered %s URLs", self.progress.state.total_discovered)

    async def _download_phase(self) -> None:
        pending = self.progress.get_failed_urls() if self.retry_failed else self.progress.get_pending_urls()
        total = len(pending)
        if total == 0:
            logger.info("No URLs to download")
            return

        logger.info("Downloading %s pages with %s workers...", total, self.workers)
        self.display.start_download(total)

        connector = aiohttp.TCPConnector(limit=self.workers)
        async with aiohttp.ClientSession(connector=connector, headers=DEFAULT_HEADERS) as session:
            tasks = [self._process_url(session, entry, total) for entry in pending]
            await asyncio.gather(*tasks, return_exceptions=True)

    async def _process_url(self, session: aiohttp.ClientSession, entry, total: int) -> None:
        async with self._semaphore:
            url = entry.url
            self.progress.mark_downloading(url)

            try:
                await asyncio.sleep(REQUEST_DELAY)
                html = await fetch_page(session, url)
                if html is None:
                    raise ValueError("Failed to fetch page")

                data = parse_feed_card(html, url, fallback_name=entry.feed_name_ru or "")
                name_ru = data["name_ru"] or entry.feed_name_ru or data["id"]
                name_en = self.translator.translate(name_ru, category="feed_types")
                subcategory_ru = entry.subcategory_ru or entry.region_ru or ""
                subcategory_en = self.translator.translate(subcategory_ru, category="feed_types")

                feed = FeedItem(
                    id=data["id"],
                    name=TranslatableText(ru=name_ru, en=name_en),
                    category_id=entry.category_id,
                    subcategory=TranslatableText(ru=subcategory_ru, en=subcategory_en),
                    region_id=entry.region_id,
                    nutrition=data["nutrition"],
                    source_url=url,
                    parse_errors=data.get("parse_errors", []),
                )

                async with self._io_lock:
                    self.output.add_feed(feed)
                    self.output.save_category(feed.category_id)
                    self.progress.mark_completed(url)
                    self.progress.save()
                    self._processed += 1
                    processed = self._processed
                    errors = self._errors

                self.display.update_download(processed, total, current_item=name_ru, errors=errors)
            except Exception as exc:
                error_message = str(exc)
                logger.error("Error processing %s: %s", url, error_message)
                async with self._io_lock:
                    self._errors += 1
                    self.progress.mark_failed(url, error_message)
                    self.progress.save()

    def _generate_output(self) -> None:
        logger.info("Generating output files...")
        self.output.save_all_categories()
        db_path = self.output.save_final_database(validate=self.validate)
        stats = self.output.get_stats()
        logger.info("Database saved to: %s", db_path)
        logger.info("Total feeds: %s", stats["total_feeds"])
        logger.info("Categories: %s", stats["categories"])


def main() -> int:
    """CLI entry point."""

    argument_parser = argparse.ArgumentParser(
        description="Livestock Feed Database Parser",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    argument_parser.add_argument("--resume", "-r", action="store_true", help="Resume from previous state")
    argument_parser.add_argument("--discover-only", action="store_true", help="Only discover URLs")
    argument_parser.add_argument(
        "--download-only",
        action="store_true",
        help="Only download and parse using an existing progress file",
    )
    argument_parser.add_argument(
        "--retry-failed",
        action="store_true",
        help="Retry URLs currently marked as failed in progress.json",
    )
    argument_parser.add_argument("--output", "-o", type=Path, default=OUTPUT_DIR, help="Output directory")
    argument_parser.add_argument(
        "--workers",
        "-w",
        type=int,
        default=MAX_WORKERS,
        help=f"Number of concurrent workers (default: {MAX_WORKERS})",
    )
    argument_parser.add_argument("--verbose", "-v", action="store_true", help="Enable verbose logging")
    argument_parser.add_argument("--no-validate", action="store_true", help="Skip JSON schema validation")
    argument_parser.add_argument(
        "--translate-api",
        action="store_true",
        help="Use deep-translator for terms missing from the static dictionary",
    )

    args = argument_parser.parse_args()
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    parser = FeedParser(
        workers=args.workers,
        resume=args.resume or args.download_only,
        discover_only=args.discover_only,
        download_only=args.download_only,
        retry_failed=args.retry_failed,
        output_dir=args.output,
        verbose=args.verbose,
        validate=not args.no_validate,
        translate_api=args.translate_api,
    )

    try:
        asyncio.run(parser.run())
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        return 1
    except Exception as exc:
        print(f"\nError: {exc}")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
