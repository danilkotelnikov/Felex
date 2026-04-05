"""URL discovery module for async crawling."""
from __future__ import annotations

import asyncio
import re
from dataclasses import dataclass
from typing import Optional
from urllib.parse import urljoin, urlparse, urlunparse

import aiohttp
from bs4 import BeautifulSoup

from config import BASE_DOMAIN, BASE_URL, DEFAULT_HEADERS, ENCODINGS, MAX_RETRIES, REQUEST_DELAY, REQUEST_TIMEOUT, RETRY_BACKOFF
from progress import ProgressDisplay, ProgressManager


REGION_NAME_TO_ID = {
    "в среднем по россии": "russia_avg",
    "северо-западный, северный экономические районы": "northwest_north",
    "центральный экономический район": "central",
    "волго-вятский экономический район": "volga_vyatka",
    "центрально-черноземный экономический район": "central_black_earth",
    "поволжский экономический район": "volga",
    "северо-кавказский экономический район": "north_caucasus",
    "уральский экономический район": "urals",
    "западно-сибирский экономический район": "west_siberia",
    "восточно-сибирский экономический район": "east_siberia",
    "дальневосточный экономический район": "far_east",
}


@dataclass(slots=True)
class DiscoveryContext:
    """Context carried through crawl levels."""

    category_id: str
    subcategory_ru: str = ""
    region_id: Optional[str] = None
    region_ru: Optional[str] = None
    section_ru: str = ""


def normalize_text(text: str) -> str:
    """Normalize human-readable text for matching."""

    return re.sub(r"\s+", " ", text.replace("\xa0", " ").strip().lower()).replace("ё", "е")


def clean_text(text: str) -> str:
    """Normalize whitespace in visible text."""

    return re.sub(r"\s+", " ", text.replace("\xa0", " ").strip())


def build_full_url(href: str, base_url: str) -> str:
    """Build a full absolute URL."""

    return normalize_site_url(urljoin(base_url, href))


def normalize_site_url(url: str) -> str:
    """Normalize the site's internal URL casing."""

    parsed = urlparse(url)
    if BASE_DOMAIN not in parsed.netloc:
        return url
    return urlunparse(parsed._replace(path=parsed.path.lower()))


def categorize_url(url: str) -> str:
    """Categorize a URL by site path."""

    path = urlparse(url).path.lower()

    if path in {"", "/"} or path.endswith("/index.html") or path.endswith("index.html"):
        return "index"
    if "/help/" in path:
        return "help"
    if "/spis/" in path or "/list/" in path:
        return "category"
    if "/vid2/" in path:
        return "region"
    if "/typ2/" in path:
        return "type"
    if "/grup/" in path:
        return "group"
    if "/komb/" in path:
        return "mixed_section"
    if "/komb1/" in path:
        return "mixed_subsection"
    if "/card/" in path or "/kkorm/" in path:
        return "card"
    if re.search(r"/[^/]+/[a-z]?\d+\.html$", path) and not re.search(
        r"/(?:spis|list|vid2|typ2|grup|help|komb|komb1)/",
        path,
    ):
        return "card"

    return "unknown"


def extract_id_from_url(url: str) -> str:
    """Extract a feed ID from a card-like URL."""

    match = re.search(r"/([a-z]?\d+)\.html$", url, re.IGNORECASE)
    if match:
        return match.group(1)
    return url.rstrip("/").split("/")[-1].replace(".html", "")


def get_category_id_from_url(url: str) -> str:
    """Map a category page URL to the database category ID."""

    category_map = {
        "11": "green_feeds",
        "12": "rough_feeds",
        "13": "succulent_feeds",
        "14": "concentrated_feeds",
        "15": "industrial_byproducts",
        "16": "animal_feeds",
        "17": "nitrogen_compounds",
        "18": "mineral_supplements",
        "19": "mixed_feeds",
    }

    match = re.search(r"[a-z](\d{2})\d{4}\.html", url, re.IGNORECASE)
    if match:
        return category_map.get(match.group(1), f"category_{match.group(1)}")
    return "unknown"


def infer_region_id(region_text: str) -> Optional[str]:
    """Infer a region ID from visible page text."""

    return REGION_NAME_TO_ID.get(normalize_text(region_text))


def extract_link_entries(html: str, base_url: str) -> list[tuple[str, str]]:
    """Extract internal links and their visible text."""

    soup = BeautifulSoup(html, "lxml")
    links: list[tuple[str, str]] = []
    seen: set[str] = set()

    for anchor in soup.find_all("a", href=True):
        href = anchor["href"].strip()
        if not href or href.startswith(("#", "mailto:", "javascript:")):
            continue

        full_url = build_full_url(href, base_url)
        parsed = urlparse(full_url)
        if parsed.scheme not in {"http", "https"}:
            continue
        if parsed.netloc and BASE_DOMAIN not in parsed.netloc:
            continue
        if "ucoz" in full_url.lower():
            continue

        text = clean_text(anchor.get_text(" ", strip=True))
        if not text:
            image = anchor.find("img")
            if image is not None:
                text = clean_text(image.get("alt", ""))

        if full_url not in seen:
            links.append((full_url, text))
            seen.add(full_url)

    return links


def extract_links_from_html(html: str, base_url: str) -> list[str]:
    """Extract internal links from HTML."""

    return [url for url, _ in extract_link_entries(html, base_url)]


async def fetch_page(session: aiohttp.ClientSession, url: str, retries: int = MAX_RETRIES) -> Optional[str]:
    """Fetch page content with retry logic and encoding detection."""

    url = normalize_site_url(url)
    timeout = aiohttp.ClientTimeout(total=REQUEST_TIMEOUT)

    for attempt in range(retries):
        try:
            async with session.get(url, timeout=timeout) as response:
                if response.status == 404:
                    return None
                if response.status != 200:
                    raise aiohttp.ClientResponseError(
                        request_info=response.request_info,
                        history=response.history,
                        status=response.status,
                        message=f"Unexpected status {response.status}",
                        headers=response.headers,
                    )

                raw = await response.read()
                encodings = []
                if response.charset:
                    encodings.append(response.charset)
                encodings.extend(ENCODINGS)

                for encoding in dict.fromkeys(encodings):
                    try:
                        return raw.decode(encoding)
                    except UnicodeDecodeError:
                        continue
                return raw.decode("utf-8", errors="replace")
        except (aiohttp.ClientError, asyncio.TimeoutError):
            if attempt < retries - 1:
                await asyncio.sleep(RETRY_BACKOFF[min(attempt, len(RETRY_BACKOFF) - 1)])

    return None


def _derive_context(current: DiscoveryContext, link_type: str, link_text: str) -> DiscoveryContext:
    text = clean_text(link_text)

    if link_type == "region":
        region_id = infer_region_id(text) or current.region_id
        return DiscoveryContext(
            category_id=current.category_id,
            subcategory_ru=current.subcategory_ru,
            region_id=region_id,
            region_ru=text or current.region_ru,
            section_ru=current.section_ru,
        )

    if link_type == "mixed_section":
        return DiscoveryContext(
            category_id=current.category_id,
            subcategory_ru=current.subcategory_ru,
            region_id=current.region_id,
            region_ru=current.region_ru,
            section_ru=text or current.section_ru,
        )

    if link_type in {"type", "group", "mixed_subsection"}:
        return DiscoveryContext(
            category_id=current.category_id,
            subcategory_ru=text or current.subcategory_ru or current.section_ru,
            region_id=current.region_id,
            region_ru=current.region_ru,
            section_ru=current.section_ru,
        )

    return current


async def _crawl_page(
    session: aiohttp.ClientSession,
    page_url: str,
    context: DiscoveryContext,
    progress_manager: ProgressManager,
    display: Optional[ProgressDisplay],
    visited_pages: set[str],
    total_found: list[int],
) -> None:
    if page_url in visited_pages:
        return

    visited_pages.add(page_url)
    await asyncio.sleep(REQUEST_DELAY)
    html = await fetch_page(session, page_url)
    if not html:
        return

    for link_url, link_text in extract_link_entries(html, page_url):
        link_type = categorize_url(link_url)

        if link_type in {"index", "help", "category"}:
            continue

        if link_type == "card":
            subcategory_ru = context.subcategory_ru or context.section_ru
            added = progress_manager.add_url(
                link_url,
                context.category_id,
                subcategory_ru,
                region_id=context.region_id,
                region_ru=context.region_ru,
                feed_name_ru=clean_text(link_text) or None,
            )
            if added:
                total_found[0] += 1
                if display:
                    display.update_discovery(total_found[0], total_found[0] + 1, clean_text(link_text))
            continue

        next_context = _derive_context(context, link_type, link_text)
        if link_type in {"region", "type", "group", "mixed_section", "mixed_subsection", "unknown"}:
            await _crawl_page(
                session,
                link_url,
                next_context,
                progress_manager,
                display,
                visited_pages,
                total_found,
            )


async def discover_all_urls(progress_manager: ProgressManager, display: Optional[ProgressDisplay] = None) -> None:
    """Discover all feed card URLs from the website."""

    if display:
        display.start_discovery()

    connector = aiohttp.TCPConnector(limit=5)
    async with aiohttp.ClientSession(connector=connector, headers=DEFAULT_HEADERS) as session:
        index_url = f"{BASE_URL}/index.html"
        index_html = await fetch_page(session, index_url)
        if not index_html:
            raise RuntimeError("Failed to fetch index page")

        category_links = [
            (url, text)
            for url, text in extract_link_entries(index_html, index_url)
            if categorize_url(url) == "category"
        ]

        visited_pages: set[str] = set()
        total_found = [0]

        for category_url, _ in category_links:
            category_id = get_category_id_from_url(category_url)
            await _crawl_page(
                session,
                category_url,
                DiscoveryContext(category_id=category_id),
                progress_manager,
                display,
                visited_pages,
                total_found,
            )

    progress_manager.set_phase("download")
    progress_manager.save()

    if display:
        display.finish_discovery(total_found[0])
