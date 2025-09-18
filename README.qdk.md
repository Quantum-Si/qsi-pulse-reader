# QDK Files

The Quantum-Si analysis software produces several data artifacts containing a summary of the workflow results.
These files typically have a ".qdk" suffix, and are zip files containing one or more parquet files with tables of values.
The following Python script can be used to read the tables into a Pandas DataFrame or to read the file's metadata:

```python
from typing import Any
import zipfile
import pandas as pd
from io import BytesIO


def read_qdk(filepath: str) -> pd.DataFrame:
    """Read QDK file into Pandas DataFrame."""
    with zipfile.ZipFile(filepath, "r") as zf:
        # Get all parquet files from the ZIP
        parquet_files = [name for name in zf.namelist() if name.endswith(".parquet")]

        # Read each parquet file and combine
        dfs = []
        for pf in parquet_files:
            with zf.open(pf) as f:
                df = pd.read_parquet(BytesIO(f.read()))
                dfs.append(df)

        # Concatenate all partitions
        return pd.concat(dfs, ignore_index=True)


def read_qdk_metadata(filepath: str) -> dict[str, Any]
    """Read the QDK file metadata."""
    with zipfile.ZipFile(filepath, "r") as zf:
        with zf.open("metadata.json") as f:
            return json.load(f)
```

For example, to read a file called `roi_calls.qdk` into a Pandas DataFrame, you would run the following command:

```python
roi_calls = read_qdk("roi_calls.qdk")
```

To extract the metadata for this file, you would run the following command:

```python
roi_calls_metadata = read_qdk_metadata("roi_calls.qdk")
```

# QDK File Specification

The columns of two of the QDK files produced by our workflows are described below.
These column definitions apply to internal versions of our workflows.
Future versions of our analysis software may have additional columns not described here.
Additionally, existing columns may be changed or removed in future versions.

## QDK file metadata

Each QDK file contains metadata describing the contents of the file. The metadata is a JSON file with the following fields:

- `'content_type'`: The type of data this file contains, e.g. `'roi_calls'` or `'alignment_results'`
- `'file_version'`: The version of this file. This field is no longer in use
- `'metadata'`: Some specific metadata, including:
  - `'run_uuids'`: A list of the UUIDs for every run used as input for the analysis producing this file
  - `'seqkit_hash'`: Optionally, a hash of the sequencing kit used in the run(s) used as input for producing this file
  - `'libkit_hash'`: Optionally, a hash of the library prep kit used in the run(s) used as input for producing this file
- `'columns'`: A list of all of the columns, their type, and whether this column is allowed to contain nulls (missing values)
- `'partition_block'`: If present, this value indicates the number of apertures contained in each parquet partition. If this value is not present, the file is not partitioned (all data is in a single parquet file)

## ROI calls file

The core results of Primary Analysis are recorded in the `roi_calls.qdk` file.
Each row of this file corresponds to a single segment of pulses that appear to result from a single peptide state.
The columns of this file are as follows (as of Primary Analysis 2.14.0):

- `ap`: The aperture index that this ROI comes from
- `ROI`: The position of this ROI within its aperture
- `start_p`: The index of the first pulse in the aperture belonging to this ROI
- `end_p`: The index of the last pulse in the aperture belonging to this ROI
- `start_f`: The frame at which this ROI starts (the first frame of the first pulse)
- `end_f`: The frame at which this ROI ends (the last frame of the last pulse)
- `start_s`: The time at which this ROI starts, in seconds
- `end_s`: The time at which this ROI ends, in seconds
- `dur_f`: The total duration of this ROI, in frames
- `dur_s`: The total duration of this ROI, in seconds
- `num_pulses`: The total number of pulses in this ROI
- `binratio_mean`: The average binratio across all pulses in this ROI
- `binratio_norm`: The sum of the pulse intensities in bin0 divided by the sum of the pulse intensities in bin1, a different way of calculating the average binratio of the ROI
- `binratio_bayes`: The uncertainty-weighted mean binratio, which gives more weight to longer pulses with more well-converged binratios and pulses with a lower background noise
- `binratio_bayes_std`: The uncertainty of `binratio_bayes`
- `pw_mean`: The average pulse duration across all pulses in this ROI
- `pw_adjusted`: A corrected version of `pw_mean` accounting for the bias introduced by short-pulse aliasing
- `pw_adjusted_median`: An estimate of the mean pulse duration obtained from the median pulse duration under the assumption of an exponential distribution of pulse durations
- `pw_map`: A corrected version of `pw_mean` accounting for biases introduced by both short-pulse aliasing and filtering of long pulses
- `ipd_mean`: The average inter-pulse duration across all pulses in this ROI
- `ipd_adjusted`: A corrected version of `ipd_mean` accounting for the bias introduced by short-inter-pulse aliasing (i.e. merging of two pulses separated by a small un-recognized gap)
- `ipd_unclip`: A corrected version of `ipd_mean` accounting for short-pulse aliasing (i.e. merging of two inter-pulse durations separated by a small un-recognized pulse)
- `snr_mean`: The average signal-to-noise ratio across all pulses in this ROI
- `intensity_mean`: The average pulse intensity (bin 1) across all pulses in this ROI
- `intensity_norm`: The duration-weighted average pulse intensity (bin 1) across all pulses in this ROI
- `intensity_bayes`: The uncertainty-weighted mean intensity, which gives more weight to longer pulses with more well-converged intensities and pulses with a lower background noise
- `intensity_bayes_std`: The uncertainty of `intensity_bayes`
- `bg_mean`: The average background intensity (bin 1) across all pulses in this ROI
- `pw_std`: The standard deviation across all pulse durations in this ROI
- `ipd_std`: The standard deviation across all inter-pulse durations in this ROI
- `snr_std`: The standard deviation across all signal-to-noise ratios in this ROI
- `intensity_std`: The standard deviationa across all pulse intensities in this ROI
- `binratio_std`: The standard deviation across all pulse binratios in this ROI
- `pw_ks`:  The Kolmogorov-Smirnov test statistic for the distribution of pulse durations in this ROI, assuming an exponential distribution as the ground truth
- `ipd_ks`: The Kolmogorov-Smirnov test statistic for the distribution of inter-pulse durations in this ROI, assuming an exponential distribution as the ground truth
- `pd_s`: The "canonical" pulse-duration column for downstream analyses, currently equal to `pw_adjusted`
- `ipd_s`: The "canonical" inter-pulse duration column for downstream analyses, currently equal to `ipd_unclip`
- `dye`: The name of the dye assigned to this binder by the dye caller
- `binder`: The name of the binder associate with the dye in the `dye` column
- `binder_residues`: The list of amino acids that the binder is capable of binding to, i.e. the candidate amino acid assignments to this ROI
- `dye_comp`: The fraction of pulses in the ROI that are well described by the properties of the dye in the `dye` column, according to the dye caller. Note that this is not a raw fraction of pulses, but uses a probabilistic assignment of pulses to each dye, so `dye_comp` is not necessarily rational
- `indeterminate_fraction`: The fraction of pulses in the ROI that are not well described by any of the dyes present in the sequencing kit, and thus are likely to be "junk"
- `binder_comp`: The fraction of pulses in the ROI that are well described by the binder in the `binder` column. Currently this is the same as `dye_comp`, since each binder is associated with a unique dye
- `roi_filters_pass`: Whether the ROI passes our internal ROI filters

# Alignment results file

The core results of Peptide Alignment are stored in the `alignment_results.qdk` file.
Each row corresponds to a single alignment with a unique read-reference pair.
When run under default settings, each read will be present only once for its highest-scoring alignments.
The columns of this file are as follows (as of Peptide Alignment 2.16.0):

- `ap`: The aperture index of the read
- `ref`: The name of the peptide reference to which the read is aligned
- `read_indices`: The indices of the ROIs participating in alignment (i.e. excluding filtered-out ROIs)
- `ref_indices`: The indices of the visible residues in the reference that align to each entry of `read_indices`. Residues for which we do not expect binding are not considered in the reference index position
- `roi_read`: The inferred sequence of amino acids associated with each aligned ROI, i.e. the NAA of each reference state in `ref_indices`
- `read_len`: The total number of ROIs participating in alignment
- `binder_read_len`: The length of the binder-collapsed read sequence, i.e. the length of the read if each adjacent ROI associated with the same binder were to be merged into a single state
- `num_binders`: The total number of unique binders present among ROIs in the read
- `num_mismatches`: The number of binder mismatches in the alignment. This is always 0 in the current version of the aligner
- `read_score`: Unused in the current version of the aligner
- `binder_match_score`: Unused in the current version of the aligner
- `match_score`: The component of the alignment score that is calculated based on the similarity of the measured ROI pulse duration and the predicted reference state pulse duration
- `gap_score`: The component of the alignment score that is calculated based on how consistent the gaps between ROIs in the read are with the number of "skipped" (invisible or deleted) residues in the alignment to the reference
- `del_score`: The component of the alignment score arising from reference states that were deleted in the final alignment trajectory
- `aln_penalty`: Unused in the current version of the aligner
- `aln_score`: The total alignment score for this read-reference pair
