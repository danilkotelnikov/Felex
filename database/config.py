"""Configuration constants for the feed parser."""
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent
DATA_DIR = PROJECT_ROOT / "data"
OUTPUT_DIR = PROJECT_ROOT / "output"
SCHEMA_DIR = PROJECT_ROOT / "schema"

for directory in (DATA_DIR, OUTPUT_DIR, SCHEMA_DIR):
    directory.mkdir(parents=True, exist_ok=True)

BASE_URL = "https://vidkormov.narod.ru"
BASE_DOMAIN = "vidkormov.narod.ru"

MAX_WORKERS = 5
REQUEST_DELAY = 1.5
REQUEST_TIMEOUT = 30
MAX_RETRIES = 3
RETRY_BACKOFF = [2, 5, 10]
ENCODINGS = ["cp1251", "windows-1251", "utf-8"]

USER_AGENT = "Mozilla/5.0 (compatible; FeedDatabaseParser/1.0)"
DEFAULT_HEADERS = {"User-Agent": USER_AGENT}

PROGRESS_FILE = DATA_DIR / "progress.json"
TRANSLATION_CACHE_FILE = DATA_DIR / "translation_cache.json"
STATIC_TRANSLATIONS_FILE = DATA_DIR / "static_translations.json"
ERRORS_LOG_FILE = PROJECT_ROOT / "errors.log"
FINAL_DB_FILE = OUTPUT_DIR / "feeds_database.json"
SCHEMA_FILE = SCHEMA_DIR / "feed_database.schema.json"
