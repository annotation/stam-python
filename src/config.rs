use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::*;

use stam::*;
use stamtools::align::{AbsoluteOrRelative, AlignmentAlgorithm, AlignmentConfig};

pub fn get_config(kwargs: &PyDict) -> Config {
    let mut config = Config::default();
    for (key, value) in kwargs {
        if let Some(key) = key.extract().unwrap() {
            match key {
                "use_include" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_use_include(value);
                    }
                }
                "debug" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_debug(value);
                    }
                }
                "workdir" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_workdir(value);
                    }
                }
                "textrelationmap" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_textrelationmap(value);
                    }
                }
                "resource_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_resource_annotation_map(value);
                    }
                }
                "dataset_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_dataset_annotation_map(value);
                    }
                }
                "annotation_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_annotation_annotation_map(value);
                    }
                }
                "key_annotation_metamap" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_key_annotation_metamap(value);
                    }
                }
                "data_annotation_metamap" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_data_annotation_metamap(value);
                    }
                }
                "generate_ids" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_generate_ids(value);
                    }
                }
                "shrink_to_fit" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_shrink_to_fit(value);
                    }
                }
                "strip_temp_ids" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_strip_temp_ids(value);
                    }
                }
                "milestone_interval" => {
                    if let Ok(Some(value)) = value.extract() {
                        config = config.with_milestone_interval(value);
                    }
                }
                _ => eprintln!("Ignored unknown kwargs option {}", key),
            }
        }
    }
    config
}

pub fn get_alignmentconfig(kwargs: &PyDict) -> PyResult<AlignmentConfig> {
    let mut alignmentconfig = AlignmentConfig::default();
    for key in kwargs.keys() {
        let key: &str = key.extract()?;
        match key {
            "case_sensitive" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<bool>() {
                        alignmentconfig.case_sensitive = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an boolean",
                            key
                        )));
                    }
                }
            }
            "trim" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<bool>() {
                        alignmentconfig.trim = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an boolean",
                            key
                        )));
                    }
                }
            }
            "simple_only" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<bool>() {
                        alignmentconfig.simple_only = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an boolean",
                            key
                        )));
                    }
                }
            }
            "algorithm" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<&str>() {
                        alignmentconfig.algorithm = match value {
                            "needlemanwunsch" | "NeedlemanWunsch" | "global" => {
                                AlignmentAlgorithm::NeedlemanWunsch {
                                    equal: 1,
                                    align: -1,
                                    insert: -1,
                                    delete: -1,
                                }
                            }
                            "smithwaterman" | "SmithWaterman" | "local" => {
                                AlignmentAlgorithm::default()
                            }
                            _ => {
                                return Err(PyValueError::new_err(
                                    "Algorithm must be 'needlemanwunsch' or 'smithwaterman'",
                                ))
                            }
                        };
                    }
                }
            }
            "annotation_id_prefix" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<String>() {
                        alignmentconfig.annotation_id_prefix = Some(value);
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be a string",
                            key
                        )));
                    }
                }
            }
            "max_errors" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<usize>() {
                        alignmentconfig.max_errors = Some(AbsoluteOrRelative::Absolute(value));
                    } else if let Ok(value) = value.extract::<f64>() {
                        alignmentconfig.max_errors = Some(AbsoluteOrRelative::Relative(value));
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an integer (absolute value) or float (relative value)",
                            key
                        )));
                    }
                }
            }
            "minimal_align_length" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<usize>() {
                        alignmentconfig.minimal_align_length = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an integer",
                            key
                        )));
                    }
                }
            }
            "grow" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<bool>() {
                        alignmentconfig.grow = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an boolean",
                            key
                        )));
                    }
                }
            }
            "verbose" | "debug" => {
                if let Ok(Some(value)) = kwargs.get_item(key) {
                    if let Ok(value) = value.extract::<bool>() {
                        alignmentconfig.verbose = value;
                    } else {
                        return Err(PyValueError::new_err(format!(
                            "Keyword argument {} must be an boolean",
                            key
                        )));
                    }
                }
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown keyword argument for align_text: {}",
                    other
                )))
            }
        }
    }
    Ok(alignmentconfig)
}
