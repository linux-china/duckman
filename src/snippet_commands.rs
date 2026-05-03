use crate::duckman_config::{DuckmanConfig, duckman_home_dir};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;

struct Snippet {
    name: String,
    front_matter: HashMap<String, String>,
    tags: Vec<String>,
    body: String,
}

fn parse_tags(value: &str) -> Vec<String> {
    // supports: [tag1, tag2] or tag1, tag2
    let trimmed = value.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .map(|t| t.replace(&[' ', '"'], ""))
        .collect()
}

fn parse_snippet(name: &str, content: &str) -> Snippet {
    let mut front_matter = HashMap::new();
    let mut tags = Vec::new();
    let body;

    if content.starts_with("---") {
        let rest = &content[3..];
        if let Some(end) = rest.find("\n---") {
            let fm_text = &rest[..end];
            for line in fm_text.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim().to_string();
                    let val = v.trim().to_string();
                    if key == "tags" {
                        tags = parse_tags(&val);
                    } else {
                        front_matter.insert(key, val);
                    }
                }
            }
            body = rest[end + 4..].trim_start_matches('\n').to_string();
        } else {
            body = content.to_string();
        }
    } else {
        body = content.to_string();
    }

    Snippet {
        name: name.to_string(),
        front_matter,
        tags,
        body,
    }
}

pub fn list_snippets() -> anyhow::Result<()> {
    let dir = DuckmanConfig::snippets_dir();
    if !dir.exists() {
        println!("No snippets found ({})", dir.display());
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No snippets found.");
        return Ok(());
    }

    println!("Snippets: {}", entries.len().to_string().green());
    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path).unwrap_or_default();
        let snippet = parse_snippet(&name, &content);
        let summary = snippet
            .front_matter
            .get("summary")
            .map(|s| s.as_str())
            .unwrap_or("");
        let tags_str = if snippet.tags.is_empty() {
            "".to_owned()
        } else {
            format!("  [{}]", snippet.tags.join(", ").cyan().to_string())
        };
        println!(
            "  {}. {}  {}{}",
            (i + 1).to_string().dimmed(),
            name.green(),
            summary.dimmed(),
            tags_str
        );
    }

    Ok(())
}

pub fn show_snippet(name: &str) -> anyhow::Result<()> {
    let path = if name.ends_with(".md") {
        DuckmanConfig::snippets_dir().join(name)
    } else {
        DuckmanConfig::snippets_dir().join(format!("{}.md", name))
    };
    if !path.exists() {
        anyhow::bail!("Snippet '{}' not found.", name);
    }

    let content = fs::read_to_string(&path)?;
    let snippet = parse_snippet(name, &content);

    // title
    println!("{}", snippet.name.green().bold());
    if let Some(summary) = snippet.front_matter.get("summary") {
        println!("{}", summary.dimmed());
    }
    if !snippet.tags.is_empty() {
        println!("tags: {}", snippet.tags.join(", ").cyan());
    }
    for (k, v) in &snippet.front_matter {
        if k != "summary" {
            println!("{}: {}", k.cyan(), v);
        }
    }
    println!();

    // body: render code blocks with highlighting, pass through other lines
    let mut in_code = false;
    let mut lang = String::new();
    for line in snippet.body.lines() {
        if line.starts_with("```") {
            if in_code {
                println!("{}", "```".dimmed());
                in_code = false;
                lang.clear();
            } else {
                lang = line[3..].trim().to_string();
                println!("{}", format!("```{}", lang).dimmed());
                in_code = true;
            }
        } else if in_code {
            println!("{}", line.yellow());
        } else {
            println!("{}", line);
        }
    }

    Ok(())
}

pub fn edit_snippet(name: &str) -> anyhow::Result<()> {
    let dir = DuckmanConfig::snippets_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.md", name));

    if !path.exists() {
        fs::write(
            &path,
            format!(
                "---\nsummary: \ntags: []\n---\n\n```sql\n-- {}\n```\n",
                name
            ),
        )?;
        println!("Created new snippet: {}", name.green());
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to launch editor '{}': {}", editor, e))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}
