from itertools import pairwise

import numpy as np
import pandas as pd
import pytest

from qsi_pulse_reader import PulseFilter, PulseReader, merge_pulse_files


def test_pulse_reader(pulse_reader):
    for ap in pulse_reader.apertures:
        # Read records and pulses and check that the aperture index attribute is correct
        records = pulse_reader.get_all_records(ap)
        assert records.attrs["aperture_index"] == ap

        pulses = pulse_reader.get_pulses(ap)
        assert pulses.attrs["aperture_index"] == ap

        # Pulses are a subset of records, check that the numbers line up
        assert len(records) >= len(pulses)

        # Extract records corresponding to pulses and check that they are all of type "pulse"
        pulse_records = records.loc[pulses.index]
        assert len(pulse_records) == len(pulses)
        assert np.all(pulse_records["record_type"] == "pulse")

        # Check that the frames-to-seconds conversion is correct
        np.testing.assert_allclose(pulses["dur_f"] * pulses.attrs["frame_dur_s"], pulses["dur_s"])
        np.testing.assert_approx_equal(
            pulses.attrs["run_dur_f"] * pulses.attrs["frame_dur_s"],
            pulses.attrs["run_dur_s"],
            significant=6,
        )


@pytest.mark.parametrize(
    "pulse_filter_kwargs",
    [
        {"min_dur_f": 10},
        {"min_dur_s": 1.0, "max_dur_s": 2.0},
        {"min_snr": 6.0},
        {"min_intensity": 100.0},
        {"min_binratio": 0.1, "max_binratio": 0.8},
        {"start_m": 120, "end_m": 240},
        {"mask_s": (3600.0, 7200.0)},
        {"min_dur_f": 10, "recalc_ipd": True},
    ],
)
def test_pulse_filter(pulse_file, pulse_reader, pulse_filter_kwargs):
    pulse_filter = PulseFilter(**pulse_filter_kwargs)

    filtered_pulse_reader_1 = PulseReader(pulse_file, pulse_filter=pulse_filter)
    filtered_pulse_reader_2 = PulseReader(pulse_file, pulse_filter_kwargs=pulse_filter_kwargs)

    for ap in filtered_pulse_reader_1.apertures:
        filtered_pulses_1 = filtered_pulse_reader_1.get_pulses(ap)
        filtered_pulses_2 = filtered_pulse_reader_2.get_pulses(ap)
        filtered_pulses_3 = pulse_reader.get_pulses(ap, pulse_filter=pulse_filter)
        filtered_pulses_4 = pulse_reader.get_pulses(ap, pulse_filter_kwargs=pulse_filter_kwargs)
        pd.testing.assert_frame_equal(filtered_pulses_1, filtered_pulses_2)
        pd.testing.assert_frame_equal(filtered_pulses_1, filtered_pulses_3)
        pd.testing.assert_frame_equal(filtered_pulses_1, filtered_pulses_4)

        spf = filtered_pulses_1.attrs["frame_dur_s"]

        assert np.all(filtered_pulses_1["dur_f"] >= pulse_filter_kwargs.get("min_dur_f", 0))
        assert np.all(filtered_pulses_1["dur_s"] >= pulse_filter_kwargs.get("min_dur_s", 0))
        assert np.all(filtered_pulses_1["dur_s"] <= pulse_filter_kwargs.get("max_dur_s", np.inf))
        assert np.all(filtered_pulses_1["snr"] >= pulse_filter_kwargs.get("min_snr", 0))
        assert np.all(
            filtered_pulses_1["intensity"] >= pulse_filter_kwargs.get("min_intensity", -np.inf)
        )
        assert np.all(
            filtered_pulses_1["binratio"] >= pulse_filter_kwargs.get("min_binratio", -np.inf)
        )
        assert np.all(
            filtered_pulses_1["binratio"] <= pulse_filter_kwargs.get("max_binratio", np.inf)
        )
        assert np.all(
            filtered_pulses_1["start_f"] * spf / 60 >= pulse_filter_kwargs.get("start_m", -np.inf)
        )
        assert np.all(
            filtered_pulses_1["end_f"] * spf / 60 <= pulse_filter_kwargs.get("end_m", np.inf)
        )

        mask_start, mask_end = pulse_filter_kwargs.get("mask_s", (np.inf, np.inf))
        assert np.all(
            (filtered_pulses_1["start_f"] * spf <= mask_start)
            | (filtered_pulses_1["start_f"] * spf >= mask_end)
        )

        if pulse_filter_kwargs.get("recalc_ipd", False):
            for pulse_a, pulse_b in pairwise(filtered_pulses_1.itertuples()):
                assert pulse_a.end_f + pulse_b.ipd_f == pulse_b.start_f


def test_copy_apertures_to_new_file(pulse_reader, tmp_path):
    new_file = str(tmp_path / "new_pulses.bin")
    apertures_to_copy = pulse_reader.apertures[:5]
    pulse_reader.copy_apertures_to_new_file(apertures_to_copy, new_file)

    new_pulse_reader = PulseReader(new_file)
    assert new_pulse_reader.apertures == apertures_to_copy

    for ap in apertures_to_copy:
        original_pulses = pulse_reader.get_pulses(ap)
        new_pulses = new_pulse_reader.get_pulses(ap)
        pd.testing.assert_frame_equal(original_pulses, new_pulses)


@pytest.mark.parametrize("n_copies", [1, 2, 3])
def test_merge_pulse_files(pulse_file, tmp_path, n_copies):
    new_file = str(tmp_path / "merged_pulses.bin")
    pulse_files = [pulse_file] * n_copies
    pulse_reader = PulseReader(pulse_file)

    merge_pulse_files(pulse_files, new_file)
    merged_pulse_reader = PulseReader(new_file)

    cols = merged_pulse_reader.metadata["cols"]
    ap_offset = 0
    tot_aps = 0
    tot_rows = 0
    for _ in range(n_copies):
        tot_aps += len(pulse_reader.apertures)
        tot_rows += pulse_reader.metadata["rows"]

        for ap in pulse_reader.apertures:
            new_records = merged_pulse_reader.get_all_records(ap + ap_offset)
            ap_index = new_records.attrs["aperture_index"]
            assert (
                ap_index == ap + ap_offset
            ), f"Aperture index mismatch for {ap}: expected {ap + ap_offset}, got {ap_index}"

            x = new_records.attrs["aperture_x"]
            y = new_records.attrs["aperture_y"]
            assert ap_index == x + cols * y

            old_records = pulse_reader.get_all_records(ap)
            assert old_records.equals(new_records)

        ap_offset += pulse_reader.metadata["cols"] * pulse_reader.metadata["rows"]

    assert len(merged_pulse_reader.apertures) == tot_aps
    assert merged_pulse_reader.metadata["rows"] == tot_rows
