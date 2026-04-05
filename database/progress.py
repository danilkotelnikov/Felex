"""Progress tracking and management."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Optional

from rich.console import Console
from rich.panel import Panel
from rich.progress import BarColumn, Progress, SpinnerColumn, TaskProgressColumn, TextColumn, TimeRemainingColumn
from rich.table import Table

from config import PROGRESS_FILE
from models import ProgressState, UrlEntry


class ProgressManager:
    """Manages scraping progress with persistence."""

    def __init__(self, progress_file: Optional[Path] = None, reset: bool = False):
        self.progress_file = progress_file or PROGRESS_FILE
        self.state = ProgressState() if reset else self._load_state()
        self._url_index: dict[str, int] = {}
        self._rebuild_index()
        self._recalculate_totals()

    def _load_state(self) -> ProgressState:
        if self.progress_file.exists():
            with open(self.progress_file, "r", encoding="utf-8") as file:
                return ProgressState.model_validate(json.load(file))
        return ProgressState()

    def _rebuild_index(self) -> None:
        self._url_index = {entry.url: index for index, entry in enumerate(self.state.discovered_urls)}

    def _recalculate_totals(self) -> None:
        self.state.total_discovered = len(self.state.discovered_urls)
        self.state.total_parsed = sum(1 for entry in self.state.discovered_urls if entry.status == "parsed")
        self.state.total_failed = sum(1 for entry in self.state.discovered_urls if entry.status == "failed")

    def save(self) -> None:
        self.progress_file.parent.mkdir(parents=True, exist_ok=True)
        with open(self.progress_file, "w", encoding="utf-8") as file:
            json.dump(self.state.model_dump(), file, ensure_ascii=False, indent=2)

    def add_url(
        self,
        url: str,
        category_id: str,
        subcategory_ru: str,
        region_id: Optional[str] = None,
        region_ru: Optional[str] = None,
        feed_name_ru: Optional[str] = None,
    ) -> bool:
        if url in self._url_index:
            return False

        entry = UrlEntry(
            url=url,
            category_id=category_id,
            subcategory_ru=subcategory_ru,
            region_id=region_id,
            region_ru=region_ru,
            feed_name_ru=feed_name_ru,
            status="pending",
        )
        self.state.discovered_urls.append(entry)
        self._url_index[url] = len(self.state.discovered_urls) - 1
        self.state.total_discovered = len(self.state.discovered_urls)
        return True

    def mark_downloading(self, url: str) -> None:
        if url in self._url_index:
            entry = self.state.discovered_urls[self._url_index[url]]
            if entry.status == "failed" and self.state.total_failed > 0:
                self.state.total_failed -= 1
            elif entry.status == "parsed" and self.state.total_parsed > 0:
                self.state.total_parsed -= 1
            entry.status = "downloading"

    def mark_completed(self, url: str) -> None:
        if url not in self._url_index:
            return

        entry = self.state.discovered_urls[self._url_index[url]]
        if entry.status != "parsed":
            entry.status = "parsed"
            entry.error_message = None
            self.state.total_parsed += 1

    def mark_failed(self, url: str, error_message: str) -> None:
        if url not in self._url_index:
            return

        entry = self.state.discovered_urls[self._url_index[url]]
        if entry.status == "parsed" and self.state.total_parsed > 0:
            self.state.total_parsed -= 1
        if entry.status != "failed":
            self.state.total_failed += 1
        entry.status = "failed"
        entry.error_message = error_message

    def get_pending_urls(self) -> list[UrlEntry]:
        return [entry for entry in self.state.discovered_urls if entry.status in {"pending", "downloading"}]

    def get_failed_urls(self) -> list[UrlEntry]:
        return [entry for entry in self.state.discovered_urls if entry.status == "failed"]

    def set_phase(self, phase: str) -> None:
        self.state.phase = phase
        self.save()

    def get_stats(self) -> dict:
        return {
            "phase": self.state.phase,
            "total": self.state.total_discovered,
            "parsed": self.state.total_parsed,
            "failed": self.state.total_failed,
            "pending": max(
                self.state.total_discovered - self.state.total_parsed - self.state.total_failed,
                0,
            ),
        }


class ProgressDisplay:
    """Rich console progress display."""

    def __init__(self, console: Optional[Console] = None):
        self.console = console or Console()
        self._rich_enabled = "utf" in (sys.stdout.encoding or "").lower()
        self._progress: Optional[Progress] = None
        self._discovery_task: Optional[int] = None
        self._download_task: Optional[int] = None

    def create_progress(self) -> Progress:
        return Progress(
            SpinnerColumn(),
            TextColumn("[bold blue]{task.description}"),
            BarColumn(),
            TaskProgressColumn(),
            TextColumn("[cyan]{task.fields[status]}"),
            TimeRemainingColumn(),
            console=self.console,
        )

    def _ensure_started(self) -> None:
        if not self._rich_enabled:
            return
        if self._progress is None:
            self.console.print(Panel.fit("[bold green]Feed Database Parser v1.0[/]", border_style="green"))
            self._progress = self.create_progress()
            self._progress.start()

    def start_discovery(self, total: Optional[int] = None) -> None:
        if not self._rich_enabled:
            print("Feed Database Parser v1.0")
            return
        self._ensure_started()
        if self._discovery_task is None and self._progress is not None:
            self._discovery_task = self._progress.add_task(
                "[Phase 1: Discovery]",
                total=total or 100,
                status="Scanning...",
            )

    def update_discovery(self, completed: int, total: int, current: str = "") -> None:
        if not self._rich_enabled:
            return
        if self._progress is not None and self._discovery_task is not None:
            status = f"{completed}/{total} URLs"
            if current:
                status = f"{status} | {current[:40]}"
            self._progress.update(self._discovery_task, completed=completed, total=max(total, 1), status=status)

    def finish_discovery(self, total: int) -> None:
        if not self._rich_enabled:
            return
        if self._progress is not None and self._discovery_task is not None:
            self._progress.update(self._discovery_task, completed=total, total=max(total, 1), status=f"{total} URLs found")

    def start_download(self, total: int) -> None:
        if not self._rich_enabled:
            return
        self._ensure_started()
        if self._progress is not None:
            self._download_task = self._progress.add_task("[Phase 2: Download]", total=max(total, 1), status="Starting...")

    def update_download(
        self,
        completed: int,
        total: int,
        current_item: str = "",
        workers: int = 0,
        errors: int = 0,
        cache_hits: int = 0,
    ) -> None:
        if not self._rich_enabled:
            return
        if self._progress is None or self._download_task is None:
            return

        status = f"{completed}/{total}"
        if errors:
            status += f" | Errors: {errors}"
        if current_item:
            status += f" | {current_item[:40]}"
        self._progress.update(self._download_task, completed=completed, total=max(total, 1), status=status)

    def finish(self) -> None:
        if not self._rich_enabled:
            return
        if self._progress is not None:
            self._progress.stop()
            self._progress = None
            self._discovery_task = None
            self._download_task = None

    def print_summary(self, stats: dict) -> None:
        if not self._rich_enabled:
            print(f"Phase: {stats.get('phase', '')}")
            print(f"Total URLs: {stats.get('total', 0)}")
            print(f"Parsed: {stats.get('parsed', 0)}")
            print(f"Failed: {stats.get('failed', 0)}")
            print(f"Pending: {stats.get('pending', 0)}")
            return
        table = Table(title="Parsing Complete", border_style="green")
        table.add_column("Metric", style="cyan")
        table.add_column("Value", style="green")
        table.add_row("Phase", str(stats.get("phase", "")))
        table.add_row("Total URLs", str(stats.get("total", 0)))
        table.add_row("Parsed", str(stats.get("parsed", 0)))
        table.add_row("Failed", str(stats.get("failed", 0)))
        table.add_row("Pending", str(stats.get("pending", 0)))
        self.console.print(table)
