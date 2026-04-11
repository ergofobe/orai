#[allow(dead_code)]
pub fn get_server_tools(
    include_web_search: bool,
    include_datetime: bool,
) -> Vec<serde_json::Value> {
    let mut tools = Vec::new();

    if include_web_search {
        tools.push(serde_json::json!({
            "type": "openrouter:web_search"
        }));
    }

    if include_datetime {
        tools.push(serde_json::json!({
            "type": "openrouter:datetime"
        }));
    }

    tools
}

#[allow(dead_code)]
pub fn get_server_tools_with_config(
    include_web_search: bool,
    include_datetime: bool,
    search_engine: &str,
    max_search_results: u32,
) -> Vec<serde_json::Value> {
    let mut tools = Vec::new();

    if include_web_search {
        let mut web_search = serde_json::json!({
            "type": "openrouter:web_search"
        });

        if search_engine != "auto" || max_search_results != 5 {
            let mut params = serde_json::Map::new();
            if search_engine != "auto" {
                params.insert(
                    "engine".to_string(),
                    serde_json::Value::String(search_engine.to_string()),
                );
            }
            if max_search_results != 5 {
                params.insert(
                    "max_results".to_string(),
                    serde_json::Value::Number(max_search_results.into()),
                );
            }
            web_search["parameters"] = serde_json::Value::Object(params);
        }

        tools.push(web_search);
    }

    if include_datetime {
        tools.push(serde_json::json!({
            "type": "openrouter:datetime"
        }));
    }

    tools
}
