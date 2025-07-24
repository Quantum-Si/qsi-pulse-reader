from pathlib import Path

import pytest

from qsi_pulse_reader import PulseReader

TEST_DIR = Path(__file__).parent.resolve()


@pytest.fixture
def pulse_file():
    return str(TEST_DIR / "pulses.bin")


@pytest.fixture
def pulse_reader(pulse_file):
    return PulseReader(pulse_file)
