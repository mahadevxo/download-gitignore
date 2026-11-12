use std::collections::BTreeSet;
use std::process::{Command, ExitCode};

use clap::{CommandFactory, Parser};

const CONTENTS_URL: &str = "https://api.github.com/repos/github/gitignore/contents";

/// Download .gitignore templates from github.com/github/gitignore
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Download a combined .gitignore for LANGUAGES (e.g. python c c++)
    #[arg(long, num_args = 0.., value_name = "LANGUAGES")]
    download: Option<Vec<String>>,

    /// Open the .gitignore template for LANGUAGE in vim
    #[arg(long, value_name = "LANGUAGE")]
    show: Option<String>,

    /// Detect languages used in the current directory (combine with --download)
    #[arg(long)]
    languages: bool,

    /// List all available templates
    #[arg(long)]
    list: bool,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Template {
    name: String, // e.g. "C++" (file name without .gitignore)
    url: String,
}

fn get(url: &str) -> Result<String> {
    Ok(ureq::get(url)
        .header("User-Agent", "dgi")
        .call()?
        .body_mut()
        .read_to_string()?)
}

fn templates() -> Result<Vec<Template>> {
    let json: serde_json::Value = serde_json::from_str(&get(CONTENTS_URL)?)?;
    let entries = json.as_array().ok_or("unexpected GitHub API response")?;
    Ok(entries
        .iter()
        .filter(|e| e["type"] == "file")
        .filter_map(|e| {
            let name = e["name"].as_str()?.strip_suffix(".gitignore")?;
            Some(Template {
                name: name.to_string(),
                url: e["download_url"].as_str()?.to_string(),
            })
        })
        .collect())
}

/// Resolve user-supplied names (case-insensitive) to templates, failing on any unknown name.
fn resolve<'a>(all: &'a [Template], langs: &[String]) -> Result<Vec<&'a Template>> {
    langs
        .iter()
        .map(|lang| {
            all.iter()
                .find(|t| t.name.eq_ignore_ascii_case(lang))
                .ok_or_else(|| format!("no .gitignore found for '{lang}' (see dgi --list)").into())
        })
        .collect()
}

fn download(langs: &[String]) -> Result<()> {
    let all = templates()?;
    let mut out = String::new();
    let mut seen = BTreeSet::new();
    for t in resolve(&all, langs)?.into_iter().filter(|t| seen.insert(&t.name)) {
        println!("Downloading {}.gitignore", t.name);
        out.push_str(&format!("### {} ###\n", t.name));
        out.push_str(&get(&t.url)?);
        out.push_str("\n\n");
    }

    let Ok(existing) = std::fs::read_to_string(".gitignore") else {
        std::fs::write(".gitignore", out)?;
        println!("Wrote .gitignore");
        return Ok(());
    };

    print_diff(&existing, &out);
    match prompt("A .gitignore already exists. [c]ombine, [o]verwrite or [a]bort? [c/o/A] ")?.as_str() {
        "c" | "combine" => {
            std::fs::write(".gitignore", combine(&existing, &out))?;
            println!("Combined into .gitignore");
        }
        "o" | "overwrite" => {
            std::fs::write(".gitignore", out)?;
            println!("Overwrote .gitignore");
        }
        _ => println!("Aborted, .gitignore unchanged"),
    }
    Ok(())
}

/// Append `new` to `existing`, dropping rules that `existing` already has.
fn combine(existing: &str, new: &str) -> String {
    let have: BTreeSet<&str> = existing.lines().map(str::trim).collect();
    let mut out = existing.trim_end().to_string();
    out.push_str("\n\n");
    for line in new.lines() {
        let t = line.trim();
        let is_rule = !t.is_empty() && !t.starts_with('#');
        if !(is_rule && have.contains(t)) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Print a colored unified diff from the existing .gitignore to the downloaded one.
fn print_diff(old: &str, new: &str) {
    let diff = similar::TextDiff::from_lines(old, new);
    for hunk in diff.unified_diff().header(".gitignore (existing)", ".gitignore (downloaded)").iter_hunks() {
        let text = hunk.to_string();
        for line in text.lines() {
            match line.chars().next() {
                Some('+') => println!("\x1b[32m{line}\x1b[0m"),
                Some('-') => println!("\x1b[31m{line}\x1b[0m"),
                Some('@') => println!("\x1b[36m{line}\x1b[0m"),
                _ => println!("{line}"),
            }
        }
    }
}

fn prompt(msg: &str) -> Result<String> {
    use std::io::Write;
    print!("{msg}");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().to_lowercase())
}

fn show(lang: &str) -> Result<()> {
    let all = templates()?;
    let t = resolve(&all, &[lang.to_string()])?[0];
    let path = std::env::temp_dir().join(format!("{}.gitignore", t.name));
    std::fs::write(&path, get(&t.url)?)?;
    let status = Command::new("vim").arg("-R").arg(&path).status()?;
    let _ = std::fs::remove_file(&path);
    if !status.success() {
        return Err("vim exited with an error".into());
    }
    Ok(())
}

/// Map a file extension to a github/gitignore template name.
fn language_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "py" | "pyi" | "ipynb" => "Python",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "C++",
        "rs" => "Rust",
        "go" => "Go",
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => "Node",
        "java" | "kt" | "kts" => "Java",
        "cs" => "VisualStudio",
        "swift" => "Swift",
        "m" | "mm" => "Objective-C",
        "rb" => "Ruby",
        "dart" => "Dart",
        "hs" => "Haskell",
        "ex" | "exs" => "Elixir",
        "erl" => "Erlang",
        "scala" => "Scala",
        "r" | "R" => "R",
        "jl" => "Julia",
        "lua" => "Lua",
        "zig" => "Zig",
        "pl" | "pm" => "Perl",
        "ml" | "mli" => "OCaml",
        "f90" | "f95" | "f03" => "Fortran",
        "elm" => "Elm",
        "nim" => "Nim",
        "d" => "D",
        "tex" => "TeX",
        _ => return None,
    })
}

/// Walk the current directory (respecting .gitignore) and collect detected languages.
fn detect_languages() -> BTreeSet<String> {
    ignore::WalkBuilder::new(".")
        .require_git(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|e| {
            let ext = e.path().extension()?.to_str()?;
            language_for(ext).map(String::from)
        })
        .collect()
}

fn run(cli: Cli) -> Result<()> {
    if cli.list {
        for t in templates()? {
            println!("{}", t.name);
        }
    }

    if let Some(lang) = &cli.show {
        show(lang)?;
    }

    let mut langs = Vec::new();
    if cli.languages {
        let detected = detect_languages();
        if detected.is_empty() {
            return Err("no known languages detected".into());
        }
        println!("Detected: {}", detected.iter().cloned().collect::<Vec<_>>().join(" "));
        langs.extend(detected);
    }

    if let Some(extra) = cli.download {
        langs.extend(extra);
        if langs.is_empty() {
            return Err("--download needs languages (or use --languages --download)".into());
        }
        download(&langs)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.download.is_none() && cli.show.is_none() && !cli.languages && !cli.list {
        Cli::command().print_help().unwrap();
        return ExitCode::SUCCESS;
    }
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
