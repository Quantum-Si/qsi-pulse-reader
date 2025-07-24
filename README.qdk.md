# QDK Files

Some of the output files produced by our data analysis pipeline have ".qdk" suffixes.
These are zip files containing one or more parquet files with tables of values.
The following Python script can be used to read these files as a Pandas DataFrame:

```python
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
```

For example, to read a file called `roi_calls.qdk` into a Pandas DataFrame, you would run the following command:

```python
roi_calls = read_qdk("roi_calls.qdk")
```
