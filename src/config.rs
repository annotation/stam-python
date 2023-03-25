use pyo3::types::*;

use stam::*;

pub fn get_config(kwargs: &PyDict) -> Config {
    let mut config = Config::default();
    for (key, value) in kwargs {
        if let Some(key) = key.extract().unwrap() {
            match key {
                "use_include" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.use_include = value;
                    }
                }
                "debug" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.debug = value;
                    }
                }
                "textrelationmap" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.textrelationmap = value;
                    }
                }
                "resource_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.resource_annotation_map = value;
                    }
                }
                "dataset_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.dataset_annotation_map = value;
                    }
                }
                "annotation_annotation_map" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.annotation_annotation_map = value;
                    }
                }
                "generate_ids" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.generate_ids = value;
                    }
                }
                _ => eprintln!("Ignored unknown kwargs option {}", key),
            }
        }
    }
    config
}
