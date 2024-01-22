use pyo3::types::*;

use stam::*;

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
