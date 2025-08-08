# QSI Pulse Reader

The QSI Pulse Reader library provides tools for reading and processing binary pulses files created by QSI Platinum and Platinum Pro devices. It features:

- A core Rust library for high-performance binary file parsing.
- Python bindings for easy integration into data analysis workflows (with future PyPI support).
- R bindings for seamless integration into R-based data environments.

## Features

- Read and parse binary pulses files.
- Filter normalized pulse records.
- Convert records to data frames for downstream analysis.
- Cross-language support: Rust, Python, and R.

## Installation

### Rust

To include the QSI Pulse Reader core library in your Rust project, add the following to your `Cargo.toml`:

```toml
[dependencies]
qsi_pulse_reader = { git = "ssh://github.com/Quantum-Si/qsi-pulse-reader", tag = "1.1.0" }
```

### Python

The Python bindings make it simple to integrate QSI Pulse Reader into your Python projects.
Currently, Python versions 3.10 through 3.14 are supported.

The Python bindings can be installed using a Python package manager, such as pip:

```sh
pip install qsi-pulse-reader
```

### R

To use the QSI Pulse Reader in R, you can load the library using:

```r
remotes::install_github("Quantum-Si/qsi-pulse-reader", subdir = "R/qsi.pulse.reader")
library(qsi.pulse.reader)
```

## Building From Source

For developers who want to build the Python and R extensions from source, follow the instructions below. Both extensions are built using the Rust toolchain, so you must have [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.

### Build Dependencies

- **Rust & Cargo:** Install from [rustup.rs](https://rustup.rs).
- **Python Extension:**
  - Python 3.10 through 3.14 with development headers.
  - [PyO3](https://github.com/PyO3/pyo3) is used for the Python bindings.
- **R Extension:**
  - R and the R development headers.
  - [extendr](https://extendr.github.io/) for binding generation.

#### Ubuntu (or Debian-based distros)

```sh
sudo apt-get update
sudo apt-get install build-essential python3-dev libssl-dev pkg-config r-base r-base-dev
```

_For other Linux distributions, install the equivalent packages (e.g., GCC, Python development headers, SSL libraries, and R development packages)._

#### macOS

- Install [Xcode Command Line Tools](https://developer.apple.com/xcode/features/) (this provides `clang` and related tools).
- Install Rust & Cargo from [rustup.rs](https://rustup.rs).
- For the Python frontend, install Python via [Homebrew](https://brew.sh) or the official installer.
- For the R frontend, install R from [CRAN](https://cran.r-project.org).

#### Windows

- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) (make sure to include C++ build components).
- Install Rust & Cargo from [rustup.rs](https://rustup.rs).
- For the Python frontend, install Python from [python.org](https://www.python.org) (ensure you select the option to add Python to your PATH, and include development headers).
  - Currently supports Python versions 3.10 through 3.14.
- For the R frontend, install R from [CRAN](https://cran.r-project.org).

### Python

1. **Install Build Requirements:**
   Make sure you have the required Python packages installed:
   ```sh
   pip install setuptools-rust build
   ```

2. **Build and install the library:**
   Run the following command from the package root
   ```sh
   pip install ./python
   ```

Note that editable installs (e.g. `pip install -e ./python`) will not automatically reflect changes to the underlying Rust code.
It is necessary to re-run `pip install ./python` in order for modifications to the Rust code to be reflected in your Python environment.

### R Extension

The R extension is available in the `R/qsi.pulse.reader/` directory and uses [extendr](https://extendr.github.io/).

1. **Change into the R directory:**

    ```sh
    cd R/qsi.pulse.reader
    ```

2. **Build and install the package:**

    From within your R console, run:

    ```r
    install.packages("devtools")  # if not already installed
    devtools::install()
    ```

3. **Testing the build:**

    You can load the package in R and run any provided examples:

    ```r
    library(qsi.pulse.reader)
    # Example usage:
    reader <- PulseReader$new("path/to/pulses.bin")
    ```

_If you are using a different Linux distro, macOS, or Windows, please install the equivalent system packages as noted above._



## Usage Examples

### Rust

```rust
use qsi_pulse_reader::pulse_reader::PulseReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pulse_reader = PulseReader::open("path/to/pulses.bin".to_string())?;
    let (pulses, aperture_header) = pulse_reader.get_pulses(0, None)?;
    // Process pulses ...
    Ok(())
}
```

### Python

```python
from qsi_pulse_reader import PulseReader, PulseFilter

# Option 1: Use keyword arguments for filtering.
pulse_reader = PulseReader("path/to/pulses.bin", pulse_filter_kwargs={"min_dur_f": 10})
valid_apertures = pulse_reader.apertures
pulses_df = pulse_reader.get_pulses(valid_apertures[0])

# Option 2: Use a PulseFilter object.
pulse_filter = PulseFilter(min_dur_f=10, min_snr=5.0)
pulse_reader = PulseReader("path/to/pulses.bin", pulse_filter=pulse_filter)
valid_apertures = pulse_reader.apertures
records_df = pulse_reader.get_all_records(valid_apertures[0])
```

### R

```r
library(qsi.pulse.reader)

# Create a new PulseReader object
reader <- PulseReader$new("path/to/pulses.bin")

# Get normalized pulses as a data frame
pulses <- reader$get_pulses(0)

# Get formatted records
records <- reader$get_all_records(0)
```

## QDK files

To learn more about how to read files with the ".qdk" suffix, check [README.qdk.md](README.qdk.md)

## Contributing

Contributions are welcome! Please submit issues and pull requests through our GitHub repository.

## License

This software is made available under the [Quantum-Si Software License Agreement](LICENSE.rst).

## Company Information

Quantum-Si brings the groundbreaking power of single-molecule protein sequencing to every lab, everywhere.
