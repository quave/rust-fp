use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

pub fn load_yaml_with_includes(path: &Path) -> Result<Yaml, Box<dyn Error>> {
    let res = process_includes_recursive(&path.to_path_buf())?;
    println!("!!Successfully processed includes: {:?}", res);
    Ok(res)
}

fn process_includes_recursive(path: &PathBuf) -> Result<Yaml, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let base_path = path.parent().unwrap_or(Path::new(""));

    let (includes, rest): (Vec<&str>, Vec<&str>) = contents
        .lines()
        .partition(|&line| line.trim().starts_with("!include"));

    let processed_includes = includes.iter().map(|&line| {
        let include_path = line.trim().strip_prefix("!include").unwrap().trim();
        let full_path = base_path.join(include_path);
        process_includes_recursive(&full_path)
            .expect(format!("Failed to process include: {}", include_path).as_str())
    });

    println!("Includes: {:?}\nrest: {:?}\n", includes, rest);

    let rest_yamls = YamlLoader::load_from_str(&rest.join("\n"))?;
    println!("Rest yamls: {:?}\n", rest_yamls);

    let merged_rest = rest_yamls
        .into_iter()
        .reduce(|acc: Yaml, include: Yaml| merge_yaml(&include, &acc))
        .ok_or("Failed to reduce includes")?;

    match processed_includes.reduce(|acc: Yaml, include: Yaml| merge_yaml(&acc, &include)) {
        Some(merged_includes) => Ok(merge_yaml(&merged_includes, &merged_rest)),
        None => Ok(merged_rest),
    }
}

fn merge_yaml(base: &Yaml, override_yaml: &Yaml) -> Yaml {
    match (base, override_yaml) {
        (Yaml::Hash(base_hash), Yaml::Hash(override_hash)) => {
            let mut result = base_hash.clone();
            for (key, value) in override_hash {
                match base_hash.get(key) {
                    Some(base_value) => {
                        result.insert(key.clone(), merge_yaml(base_value, value));
                    }
                    None => {
                        result.insert(key.clone(), value.clone());
                    }
                }
            }
            Yaml::Hash(result)
        }
        (_, override_value) => override_value.clone(),
    }
}
