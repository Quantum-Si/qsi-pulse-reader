use crate::pulse_filter::PulseFilter;
use crate::records::ToPyDict;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use qsi_pulse_reader::pulse_filter::PulseFilter as RustPulseFilter;
use qsi_pulse_reader::pulse_reader::PulseReader as RustPulseReader;
use qsi_pulse_reader::pulse_reader::headers::ApertureHeader;
use qsi_pulse_reader::pulse_reader::merge_pulse_files as rust_merge_pulse_files;

/// Pulses.bin reader
#[pyclass]
pub(super) struct PulseReader {
    pub pulse_reader: Option<RustPulseReader>,
    pulse_filter: Option<PulseFilter>,
    pandas: Py<PyModule>,
    common_attributes: Py<PyDict>,
    metadata: Py<PyDict>,
}

impl PulseReader {
    fn to_dataframe(
        &self,
        py: Python,
        header: &ApertureHeader,
        pydict: &Py<PyDict>,
    ) -> PyResult<PyObject> {
        let pandas = self.pandas.bind(py);
        let df = pandas.call_method1("DataFrame", (pydict,))?;
        let attrs = PyDict::new(py);
        attrs.set_item("aperture_index", header.well_id)?;
        attrs.set_item("aperture_x", header.x)?;
        attrs.set_item("aperture_y", header.y)?;
        attrs.set_item("aperture_byteloc", header.byte_loc)?;
        attrs.update(self.common_attributes.bind(py).as_mapping())?;
        df.setattr("attrs", attrs)?;
        Ok(df.into())
    }

    fn validate(&self) -> PyResult<()> {
        if self.pulse_reader.is_none() {
            return Err(PyRuntimeError::new_err(
                "PulseReader cannot be used after the pulse file has been closed!",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PulseReader {
    /// Open a pulses.bin file and return a PulseReader object
    ///
    /// # Arguments
    /// * `file_name` - The path to the pulses.bin file
    /// * `pulse_filter` - Optional PulseFilter object to filter the pulses
    /// * `pulse_filter_kwargs` - Optional keyword arguments for the pulse filter
    ///   (e.g., min_dur_f, min_dur_s, max_dur_s, min_snr, min_intensity, etc.)
    ///   See the PulseFilter class for more details.
    ///
    /// # Returns
    /// A PulseReader object that can be used to read and filter pulses
    /// from the specified file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or if there is an error reading the file.
    ///
    /// # Examples
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin")
    /// ```
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin", pulse_filter_kwargs={"min_dur_f": 3})
    /// ```
    /// ```python
    /// from qsi_pulse_reader import PulseReader, PulseFilter
    /// pulse_filter = PulseFilter(min_dur_f=3)
    /// pulse_reader = PulseReader("path/to/pulses.bin", pulse_filter=pulse_filter)
    /// ```
    #[new]
    #[pyo3(signature = (file_name, pulse_filter=None, pulse_filter_kwargs=None))]
    fn new(
        py: Python,
        file_name: &str,
        pulse_filter: Option<&PulseFilter>,
        pulse_filter_kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let pulse_reader = RustPulseReader::open(file_name.to_string())?;
        if pulse_filter.is_some() && pulse_filter_kwargs.is_some() {
            return Err(PyRuntimeError::new_err(
                "Cannot provide both a PulseFilter object and keyword arguments for the filter!",
            ));
        }
        let pulse_filter = if let Some(kwargs) = pulse_filter_kwargs {
            Some(PulseFilter::new(Some(kwargs))?)
        } else {
            pulse_filter.cloned()
        };
        let pandas = PyModule::import(py, "pandas")?.into();

        // Set common dataframe attributes
        let common_attributes = PyDict::new(py);
        common_attributes.set_item("source", "pulses.bin")?;
        common_attributes.set_item(
            "analysis_id",
            file_name
                .split('.')
                .next()
                .ok_or_else(|| PyRuntimeError::new_err("Invalid file name"))?,
        )?;
        common_attributes.set_item("frame_dur_s", 1.0 / pulse_reader.fps)?;
        let duration = pulse_reader.metadata["duration"].as_f64().ok_or_else(|| {
            PyRuntimeError::new_err("Missing or invalid 'duration' field in metadata")
        })? as f32;
        common_attributes.set_item("run_dur_f", (duration * pulse_reader.fps).ceil() as u64)?;
        common_attributes.set_item("run_dur_s", duration)?;

        // Load metadata
        let json = PyModule::import(py, "json")?;
        let metadata = json.call_method1("loads", (pulse_reader.raw_metadata.clone(),))?;
        let metadata = metadata.downcast::<PyDict>().unwrap().clone().into();

        Ok(PulseReader {
            pulse_reader: Some(pulse_reader),
            pulse_filter,
            pandas,
            common_attributes: common_attributes.into(),
            metadata,
        })
    }

    /// Get all formatted pulse records for a specific aperture index
    ///
    /// # Arguments
    /// * `aperture_index` - The index of the aperture to get the formatted pulse records for
    ///
    /// # Returns
    /// A pandas DataFrame containing the formatted pulse records for the specified aperture index
    ///
    /// # Examples
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin")
    /// formatted_pulses = pulse_reader.get_records(0)
    /// ```
    fn get_all_records(&mut self, py: Python, aperture_index: usize) -> PyResult<PyObject> {
        self.validate()?;
        let (records, header) = py.allow_threads(|| {
            self.pulse_reader
                .as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?
                .get_all_records(aperture_index)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to get records: {}", e)))
        })?;
        let pydict = records.to_pydict(py, None, None)?;
        let df = self.to_dataframe(py, &header, &pydict)?;
        Ok(df)
    }

    /// Get the pulse records for a specific aperture index
    ///
    /// # Arguments
    /// * `aperture_index` - The index of the aperture to get the pulse records for
    /// * `include_aperture_index` - Whether to include the aperture index in the DataFrame
    /// # * `pulse_filter_kwargs` - Optional keyword arguments for the pulse filter
    ///  (e.g., min_dur_f, min_dur_s, max_dur_s, min_snr, min_intensity, etc.)
    ///  See the PulseFilter class for more details.
    ///
    /// # Returns
    /// A pandas DataFrame containing the pulse records for the specified aperture index.
    /// The index of each pulse corresponds to the position of that pulses's record in the
    /// full set of records for the specified aperture, including non-pulse records.
    ///
    /// # Examples
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin")
    /// pulses = = pulse_reader.get_pulses(0)
    /// ```
    #[pyo3(signature = (
        aperture_index,
        include_aperture_index=true,
        pulse_filter=None,
        pulse_filter_kwargs=None,
    ))]
    fn get_pulses(
        &mut self,
        py: Python,
        aperture_index: usize,
        include_aperture_index: bool,
        pulse_filter: Option<&PulseFilter>,
        pulse_filter_kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.validate()?;
        if pulse_filter.is_some() && pulse_filter_kwargs.is_some() {
            return Err(PyRuntimeError::new_err(
                "Cannot provide both a PulseFilter object and keyword arguments for the filter!",
            ));
        }
        // Construct the new pulse filter, if provided
        let py_pulse_filter: Option<PulseFilter> = pulse_filter_kwargs
            .map(|kwargs| PulseFilter::new(Some(kwargs)))
            .transpose()?;
        // Extract the underlying Rust pulse filter. Default to the user-provided value, if present.
        // Otherwise, use the one defined at initialization.
        let pulse_filter: Option<&RustPulseFilter> = py_pulse_filter
            .as_ref()
            .map(|pf| &pf.pulse_filter)
            .or(pulse_filter.as_ref().map(|pf| &pf.pulse_filter))
            .or(self.pulse_filter.as_ref().map(|pf| &pf.pulse_filter));
        let (pulses, header) = py.allow_threads(|| {
            self.pulse_reader
                .as_mut()
                .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?
                .get_pulses(aperture_index, pulse_filter)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to get pulses: {}", e)))
        })?;
        let ap = if include_aperture_index {
            Some(aperture_index)
        } else {
            None
        };
        let pydict = pulses.to_pydict(py, ap, None)?;
        let df = self.to_dataframe(py, &header, &pydict)?;
        // Pop the column named "index", then set it as the df index.
        let index_col = df.call_method1(py, "pop", ("index",))?;
        Ok(df.call_method1(py, "set_index", (index_col,))?)
    }

    /// Copy the specified apertures to a new file
    ///
    /// # Arguments
    /// * `apertures` - A list of aperture indices to copy
    /// * `file_name` - The name of the new file to create
    /// * `ignore_missing_apertures` - Whether to ignore missing apertures
    ///
    /// # Returns
    /// A new file containing the specified apertures.
    ///
    /// # Examples
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin")
    /// pulse_reader.copy_apertures_to_new_file([0, 1], "new_pulses.bin")
    /// ```
    /// ```python
    /// from qsi_pulse_reader import PulseReader
    /// pulse_reader = PulseReader("path/to/pulses.bin")
    /// pulse_reader.copy_apertures_to_new_file([0, 1], "new_pulses.bin", ignore_missing_apertures=True)
    /// ```
    #[pyo3(signature = (apertures, file_name, ignore_missing_apertures=false))]
    fn copy_apertures_to_new_file(
        &mut self,
        apertures: Vec<usize>,
        file_name: &str,
        ignore_missing_apertures: bool,
    ) -> PyResult<()> {
        self.validate()?;

        let mut apertures = apertures.clone();
        apertures.sort_unstable();
        apertures.dedup();

        // Identify if any of the specified apertures are missing
        // Exploit the fact that both aperture lists are sorted
        let pulse_reader: &RustPulseReader = self
            .pulse_reader
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?;
        let mut found_apertures = Vec::with_capacity(apertures.len());
        let mut missing_apertures = Vec::new();
        let mut i = 0;
        let mut j = 0;
        while i < pulse_reader.index.apertures.len() && j < apertures.len() {
            if pulse_reader.index.apertures[i] == apertures[j] {
                found_apertures.push(apertures[j]);
                i += 1;
                j += 1;
            } else if pulse_reader.index.apertures[i] < apertures[j] {
                i += 1;
            } else {
                missing_apertures.push(apertures[j]);
                j += 1;
            }
        }
        if missing_apertures.len() > 0 && !ignore_missing_apertures {
            return Err(PyRuntimeError::new_err(format!(
                "The following apertures were not found in the file: {:?}",
                missing_apertures
            )));
        }

        self.pulse_reader
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?
            .copy_apertures_to_new_file(&found_apertures, file_name)?;
        Ok(())
    }

    /// Close the pulses.bin file
    fn close(&mut self) -> PyResult<()> {
        self.pulse_reader = None;
        Ok(())
    }

    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __exit__(
        &mut self,
        _exc_type: PyObject,
        _exc_value: PyObject,
        _traceback: PyObject,
    ) -> PyResult<()> {
        self.close()?;
        Ok(())
    }

    #[getter]
    fn apertures(&self) -> PyResult<Vec<usize>> {
        self.validate()?;
        Ok(self
            .pulse_reader
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?
            .index
            .apertures
            .clone())
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<Py<PyDict>> {
        Ok(self.metadata.clone_ref(py))
    }

    #[getter]
    fn trimmed(&self) -> PyResult<bool> {
        self.validate()?;
        Ok(self
            .pulse_reader
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("PulseReader is not initialized"))?
            .trimmed)
    }
}

#[pyfunction]
pub fn merge_pulse_files(file_names: Vec<String>, new_file_name: &str) -> PyResult<()> {
    let mut pulse_readers: Vec<RustPulseReader> = Vec::with_capacity(file_names.len());
    for file_name in &file_names {
        pulse_readers.push(RustPulseReader::open(file_name)?);
    }
    rust_merge_pulse_files(&mut pulse_readers, new_file_name)?;
    Ok(())
}
