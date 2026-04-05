"""Pytest configuration and fixtures."""
import shutil
import sys
import uuid
from pathlib import Path

import pytest


sys.path.insert(0, str(Path(__file__).parent.parent))


@pytest.fixture
def sample_card_html():
    """Sample feed card HTML for testing."""

    return """
    <html>
    <head><title>Жмых льняной (химический состав) - Корма России</title></head>
    <body>
    <table>
        <tr><td>Показатели</td><td>Значение</td></tr>
        <tr><td>Кормовые единицы</td><td>1,27</td></tr>
        <tr><td>Обменная энергия (КРС), МДж</td><td>11,7</td></tr>
        <tr><td>Обменная энергия (свиньи), МДж</td><td>13,73</td></tr>
        <tr><td>Сухое вещество, г</td><td>900</td></tr>
        <tr><td>Сырой протеин, г</td><td>338</td></tr>
        <tr><td>Переваримый протеин (КРС), г</td><td>287</td></tr>
        <tr><td>Лизин, г</td><td>11,5</td></tr>
        <tr><td>Кальций, г</td><td>3,4</td></tr>
        <tr><td>Фосфор, г</td><td>10</td></tr>
        <tr><td>Каротин, мг</td><td>0,3</td></tr>
    </table>
    </body>
    </html>
    """


@pytest.fixture
def sample_index_html():
    """Sample index page HTML for testing."""

    return """
    <html>
    <body>
    <ul>
        <li><a href="spis/m110000.html">Зеленые корма</a></li>
        <li><a href="spis/m120000.html">Грубые корма</a></li>
        <li><a href="list/o150000.html">Отходы промышленности</a></li>
    </ul>
    </body>
    </html>
    """


@pytest.fixture
def workspace_tmp_dir():
    """Create a temporary directory inside the repository."""

    root = Path(__file__).parent.parent / ".test_tmp"
    root.mkdir(exist_ok=True)
    path = root / uuid.uuid4().hex
    path.mkdir()
    try:
        yield path
    finally:
        shutil.rmtree(path, ignore_errors=True)


@pytest.fixture
def temp_progress_file(workspace_tmp_dir):
    """Create temporary progress file path."""

    return workspace_tmp_dir / "progress.json"
