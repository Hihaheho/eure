use clap::{Args, Parser, Subcommand, ValueEnum};
use eure_fmt::unformat::{unformat, unformat_with_seed};
use eure_json::{
    Config as JsonConfig, VariantRepr, format_eure_bindings, json_to_value_with_config,
    value_to_json_with_config,
};
use eure_tree::tree::NonTerminalHandle;
use eure_yaml::{Config as YamlConfig, value_to_yaml_with_config, yaml_to_value_with_config};
use std::fs;
use std::io::{self, Read};

#[derive(Parser)]
#[command(name = "eure", about = "EURE file utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display EURE file syntax tree
    Inspect(Inspect),
    /// Unformat EURE file
    Unformat(Unformat),
    /// Format EURE file
    Fmt(Fmt),
    /// Convert EURE to JSON
    ToJson(ToJson),
    /// Convert JSON to EURE
    FromJson(FromJson),
    /// Convert EURE to YAML
    ToYaml(ToYaml),
    /// Convert YAML to EURE
    FromYaml(FromYaml),
}

#[derive(Args)]
struct Inspect {
    /// Path to EURE file to inspect
    file: String,
}

#[derive(Args)]
struct Unformat {
    /// Path to EURE file to unformat (use '-' for stdin)
    file: Option<String>,
    /// Seed for unformatting
    #[arg(short, long)]
    seed: Option<u64>,
}

#[derive(Args)]
struct Fmt {
    /// Path to EURE file to format (use '-' for stdin)
    file: Option<String>,
    /// Check mode - exit with non-zero status if formatting is needed
    #[arg(short, long)]
    check: bool,
    /// Indent width (default: 2)
    #[arg(short, long, default_value = "2")]
    indent_width: usize,
}

#[derive(ValueEnum, Clone, Debug)]
enum VariantFormat {
    /// Default: {"variant-name": {...}}
    External,
    /// {"type": "variant-name", ...fields...}
    Internal,
    /// {"type": "variant-name", "content": {...}}
    Adjacent,
    /// Just the content without variant information
    Untagged,
}

#[derive(Args)]
struct ToJson {
    /// Path to EURE file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
    /// Pretty print JSON output
    #[arg(short, long)]
    pretty: bool,
}

#[derive(Args)]
struct FromJson {
    /// Path to JSON file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
}

#[derive(Args)]
struct ToYaml {
    /// Path to EURE file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
}

#[derive(Args)]
struct FromYaml {
    /// Path to YAML file to convert (use - for stdin)
    file: String,
    /// Variant representation format
    #[arg(short = 'v', long, value_enum, default_value = "external")]
    variant: VariantFormat,
    /// Tag field name for internal/adjacent representations
    #[arg(short = 't', long, default_value = "type")]
    tag: String,
    /// Content field name for adjacent representation
    #[arg(short = 'c', long, default_value = "content")]
    content: String,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect(Inspect { file }) => {
            let contents = match fs::read_to_string(&file) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Error reading file: {e}");
                    return;
                }
            };

            let tree = eure_parol::parse(&contents).unwrap();
            let mut out = String::new();
            tree.inspect(&contents, &mut out).unwrap();
            println!("{out}");
        }
        Commands::Unformat(Unformat { file, seed }) => {
            // Read input from file or stdin
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {e}");
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {e}");
                        std::process::exit(1);
                    }
                },
            };

            let mut tree = match eure_parol::parse(&contents) {
                Ok(tree) => tree,
                Err(e) => {
                    eprintln!("Parse error: {e:?}");
                    std::process::exit(1);
                }
            };

            if let Some(seed) = seed {
                unformat_with_seed(&mut tree, seed);
            } else {
                unformat(&mut tree);
            }

            let mut out = String::new();
            if let Err(e) = tree.write(&contents, &mut out) {
                eprintln!("Error writing output: {e}");
                std::process::exit(1);
            }
            print!("{out}");
        }
        Commands::Fmt(Fmt {
            file,
            check,
            indent_width,
        }) => {
            // Read input from file or stdin
            let contents = match file.as_deref() {
                None | Some("-") => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {e}");
                        std::process::exit(1);
                    }
                    buffer
                }
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        eprintln!("Error reading file: {e}");
                        std::process::exit(1);
                    }
                },
            };

            // Parse the input
            let mut tree = match eure_parol::parse(&contents) {
                Ok(tree) => tree,
                Err(e) => {
                    eprintln!("Parse error: {e:?}");
                    std::process::exit(1);
                }
            };

            let config = eure_fmt::FmtConfig::new(indent_width);

            if check {
                // Check mode - just verify if formatting is needed
                match eure_fmt::check_formatting_with_config(&contents, &tree, config) {
                    Ok(errors) => {
                        if !errors.is_empty() {
                            eprintln!("File needs formatting ({} issues found)", errors.len());
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking formatting: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                // Format mode - apply formatting
                if let Err(e) = eure_fmt::fmt_with_config(&contents, &mut tree, config) {
                    eprintln!("Error formatting: {e}");
                    std::process::exit(1);
                }

                let mut out = String::new();
                if let Err(e) = tree.write(&contents, &mut out) {
                    eprintln!("Error writing output: {e}");
                    std::process::exit(1);
                }
                print!("{out}");
            }
        }
        Commands::ToJson(args) => handle_to_json(args),
        Commands::FromJson(args) => handle_from_json(args),
        Commands::ToYaml(args) => handle_to_yaml(args),
        Commands::FromYaml(args) => handle_from_yaml(args),
    }
}

fn handle_to_json(args: ToJson) {
    use eure_tree::value_visitor::{ValueVisitor, Values};

    // Read input
    let contents = if args.file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading from stdin: {e}");
            return;
        }
        buffer
    } else {
        match fs::read_to_string(&args.file) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                return;
            }
        }
    };

    // Parse EURE
    let tree = match eure_parol::parse(&contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("Error parsing EURE: {e:?}");
            return;
        }
    };

    // Extract values using ValueVisitor
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(&contents, &mut values);

    // Visit the tree
    if let Err(e) = tree.visit_from_root(&mut visitor) {
        eprintln!("Error visiting EURE tree: {e:?}");
        return;
    }

    // Extract the main value from the document
    let value = if let Ok(root_view) = tree.root_handle().get_view(&tree)
        && let Some(eure_value) = values.get_eure(&root_view.eure)
    {
        eure_value.clone()
    } else {
        eprintln!("Error: Could not extract document value");
        return;
    };

    // Configure variant representation
    let variant_repr = match args.variant {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag: args.tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent {
            tag: args.tag,
            content: args.content,
        },
        VariantFormat::Untagged => VariantRepr::Untagged,
    };

    let config = JsonConfig { variant_repr };

    // Convert to JSON
    let json_value = match value_to_json_with_config(&value, &config) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error converting to JSON: {e}");
            return;
        }
    };

    // Output JSON
    let output = if args.pretty {
        match serde_json::to_string_pretty(&json_value) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error serializing JSON: {e}");
                return;
            }
        }
    } else {
        match serde_json::to_string(&json_value) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error serializing JSON: {e}");
                return;
            }
        }
    };

    println!("{output}");
}

fn handle_from_json(args: FromJson) {
    // Read input
    let contents = if args.file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading from stdin: {e}");
            return;
        }
        buffer
    } else {
        match fs::read_to_string(&args.file) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                return;
            }
        }
    };

    // Parse JSON
    let json_value: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error parsing JSON: {e}");
            return;
        }
    };

    // Configure variant representation
    let variant_repr = match args.variant {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag: args.tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent {
            tag: args.tag,
            content: args.content,
        },
        VariantFormat::Untagged => VariantRepr::Untagged,
    };

    let config = JsonConfig { variant_repr };

    // Convert to EURE Value
    let value = match json_to_value_with_config(&json_value, &config) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Error converting from JSON: {e}");
            return;
        }
    };

    // Format as EURE
    let eure_output = format_eure_bindings(&value);
    println!("{eure_output}");
}

fn handle_to_yaml(args: ToYaml) {
    use eure_tree::value_visitor::{ValueVisitor, Values};

    // Read input
    let contents = if args.file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading from stdin: {e}");
            return;
        }
        buffer
    } else {
        match fs::read_to_string(&args.file) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                return;
            }
        }
    };

    // Parse EURE
    let tree = match eure_parol::parse(&contents) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("Error parsing EURE: {e:?}");
            return;
        }
    };

    // Extract values using ValueVisitor
    let mut values = Values::default();
    let mut visitor = ValueVisitor::new(&contents, &mut values);

    // Visit the tree
    if let Err(e) = tree.visit_from_root(&mut visitor) {
        eprintln!("Error visiting EURE tree: {e:?}");
        return;
    }

    // Extract the main value from the document
    let value = if let Ok(root_view) = tree.root_handle().get_view(&tree)
        && let Some(eure_value) = values.get_eure(&root_view.eure)
    {
        eure_value.clone()
    } else {
        eprintln!("Error: Could not extract document value");
        return;
    };

    // Configure variant representation
    let variant_repr = match args.variant {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag: args.tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent {
            tag: args.tag,
            content: args.content,
        },
        VariantFormat::Untagged => VariantRepr::Untagged,
    };

    let config = YamlConfig { variant_repr };

    // Convert to YAML
    let yaml_value = match value_to_yaml_with_config(&value, &config) {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("Error converting to YAML: {e}");
            return;
        }
    };

    // Output YAML
    match serde_yaml::to_string(&yaml_value) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("Error serializing YAML: {e}");
        }
    }
}

fn handle_from_yaml(args: FromYaml) {
    // Read input
    let contents = if args.file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading from stdin: {e}");
            return;
        }
        buffer
    } else {
        match fs::read_to_string(&args.file) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading file: {e}");
                return;
            }
        }
    };

    // Parse YAML
    let yaml_value: serde_yaml::Value = match serde_yaml::from_str(&contents) {
        Ok(yaml) => yaml,
        Err(e) => {
            eprintln!("Error parsing YAML: {e}");
            return;
        }
    };

    // Configure variant representation
    let variant_repr = match args.variant {
        VariantFormat::External => VariantRepr::External,
        VariantFormat::Internal => VariantRepr::Internal { tag: args.tag },
        VariantFormat::Adjacent => VariantRepr::Adjacent {
            tag: args.tag,
            content: args.content,
        },
        VariantFormat::Untagged => VariantRepr::Untagged,
    };

    let config = YamlConfig { variant_repr };

    // Convert to EURE Value
    let value = match yaml_to_value_with_config(&yaml_value, &config) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Error converting from YAML: {e}");
            return;
        }
    };

    // Format as EURE
    let eure_output = eure_yaml::format_eure_bindings(&value);
    println!("{eure_output}");
}
