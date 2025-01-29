use clap::{Arg, ArgMatches, Command};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GhpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Profile '{0}' not found")]
    ProfileNotFound(String),
    #[error("Failed to parse config: {0}")]
    ConfigParse(String),
    #[error("Missing configuration: {0}")]
    MissingConfig(String),
}

type Result<T> = std::result::Result<T, GhpError>;

#[derive(Debug)]
struct Profile {
    username: String,
    email: String,
    ssh_key: PathBuf,
}

struct Config {
    ssh_config_path: PathBuf,
    ghp_config_path: PathBuf,
    profiles: HashMap<String, Profile>,
}

impl Config {
    fn load() -> Result<Self> {
        let default_paths = get_default_paths()?;
        let config_content = fs::read_to_string(&default_paths.1)
            .unwrap_or_else(|_| format!("ssh_config={}\nghp_config={}", 
                default_paths.0.display(), 
                default_paths.1.display()));

        let mut config = Self::parse_config(&config_content)?;
        config.ssh_config_path = default_paths.0;
        config.ghp_config_path = default_paths.1;
        Ok(config)
    }

    fn parse_config(content: &str) -> Result<Self> {
        let mut profiles = HashMap::new();
        let mut ssh_config_path = None;
        let mut ghp_config_path = None;
        let mut current_profile = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_profile = Some(line[1..line.len() - 1].to_string());
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            match (parts[0], current_profile.as_ref()) {
                ("ssh_config", None) => ssh_config_path = Some(PathBuf::from(parts[1])),
                ("ghp_config", None) => ghp_config_path = Some(PathBuf::from(parts[1])),
                ("username", Some(profile)) => {
                    profiles.entry(profile.clone())
                        .or_insert_with(|| Profile {
                            username: String::new(),
                            email: String::new(),
                            ssh_key: PathBuf::new(),
                        })
                        .username = parts[1].to_string();
                }
                ("email", Some(profile)) => {
                    profiles.entry(profile.clone())
                        .or_insert_with(|| Profile {
                            username: String::new(),
                            email: String::new(),
                            ssh_key: PathBuf::new(),
                        })
                        .email = parts[1].to_string();
                }
                ("ssh_key", Some(profile)) => {
                    profiles.entry(profile.clone())
                        .or_insert_with(|| Profile {
                            username: String::new(),
                            email: String::new(),
                            ssh_key: PathBuf::new(),
                        })
                        .ssh_key = PathBuf::from(parts[1]);
                }
                _ => {}
            }
        }

        Ok(Self {
            ssh_config_path: ssh_config_path.unwrap_or_else(|| get_default_paths().unwrap().0),
            ghp_config_path: ghp_config_path.unwrap_or_else(|| get_default_paths().unwrap().1),
            profiles,
        })
    }

    fn save(&self) -> Result<()> {
        let mut content = String::new();
        content.push_str(&format!("ssh_config={}\n", self.ssh_config_path.display()));
        content.push_str(&format!("ghp_config={}\n\n", self.ghp_config_path.display()));

        for (name, profile) in &self.profiles {
            content.push_str(&format!("[{}]\n", name));
            content.push_str(&format!("username={}\n", profile.username));
            content.push_str(&format!("email={}\n", profile.email));
            content.push_str(&format!("ssh_key={}\n\n", profile.ssh_key.display()));
        }

        fs::write(&self.ghp_config_path, content)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let matches = Command::new("ghp")
        .about("GitHub Profile Manager - Manage multiple GitHub profiles and SSH/GPG keys")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("setup")
                .about("Initial setup for paths to config files")
                .arg(
                    Arg::new("ssh_config")
                        .short('s')
                        .long("ssh-config")
                        .help("Path to the SSH config file")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("ghp_config")
                        .short('g')
                        .long("ghp-config")
                        .help("Path to store the GHP config file")
                        .value_parser(clap::value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("add")
                .about("Add a new GitHub profile")
                .arg(
                    Arg::new("profile")
                        .required(true)
                        .help("Name of the profile")
                        .value_parser(clap::value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("switch")
                .about("Switch to an existing GitHub profile")
                .arg(
                    Arg::new("profile")
                        .required(true)
                        .help("Name of the profile")
                        .value_parser(clap::value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove an existing GitHub profile")
                .arg(
                    Arg::new("profile")
                        .required(true)
                        .help("Name of the profile")
                        .value_parser(clap::value_parser!(String)),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("setup", sub_m)) => setup(sub_m),
        Some(("add", sub_m)) => add_profile(sub_m),
        Some(("switch", sub_m)) => switch_profile(sub_m),
        Some(("remove", sub_m)) => remove_profile(sub_m),
        _ => Err(GhpError::ConfigParse("Invalid subcommand".to_string())),
    }
}

fn get_default_paths() -> Result<(PathBuf, PathBuf)> {
    let home = dirs::home_dir().ok_or_else(|| GhpError::ConfigParse("Could not determine home directory".to_string()))?;
    Ok((
        home.join(".ssh").join("config"),
        home.join(".ghp"),
    ))
}

fn read_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn setup(matches: &ArgMatches) -> Result<()> {
    let ssh_config = matches.get_one::<String>("ssh_config")
        .map(PathBuf::from)
        .unwrap_or_else(|| get_default_paths().unwrap().0);
    let ghp_config = matches.get_one::<String>("ghp_config")
        .map(PathBuf::from)
        .unwrap_or_else(|| get_default_paths().unwrap().1);

    let config = Config {
        ssh_config_path: ssh_config.clone(),
        ghp_config_path: ghp_config.clone(),
        profiles: HashMap::new(),
    };
    config.save()?;

    println!("Configuration saved.");
    println!("SSH config path: {}", ssh_config.display());
    println!("GHP config path: {}", ghp_config.display());
    Ok(())
}

fn add_profile(matches: &ArgMatches) -> Result<()> {
    let profile_name = matches.get_one::<String>("profile")
        .ok_or_else(|| GhpError::MissingConfig("Profile name required".to_string()))?;
    let mut config = Config::load()?;

    let username = read_input("Enter Git username: ")?;
    let email = read_input("Enter Git email: ")?;
    let ssh_key = read_input("Enter path to SSH key: ")?;

    config.profiles.insert(profile_name.clone(), Profile {
        username: username.clone(),
        email,
        ssh_key: PathBuf::from(ssh_key.clone()),
    });

    let ssh_config = format!(
        "Host github.com-{}\n  HostName github.com\n  User {}\n  IdentityFile {}\n\n",
        profile_name, username, ssh_key
    );
    fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&config.ssh_config_path)?
        .write_all(ssh_config.as_bytes())?;

    config.save()?;
    println!("Profile '{}' added successfully!", profile_name);
    Ok(())
}

fn switch_profile(matches: &ArgMatches) -> Result<()> {
    let profile_name = matches.get_one::<String>("profile")
        .ok_or_else(|| GhpError::MissingConfig("Profile name required".to_string()))?;
    let config = Config::load()?;

    let profile = config.profiles.get(profile_name)
        .ok_or_else(|| GhpError::ProfileNotFound(profile_name.clone()))?;

    let ssh_content = fs::read_to_string(&config.ssh_config_path)
        .unwrap_or_default();
    
    let new_host_config = format!(
        "Host github.com\n  HostName github.com\n  User {}\n  IdentityFile {}\n",
        profile.username,
        profile.ssh_key.display()
    );

    let updated_content = update_github_host_in_ssh_config(&ssh_content, &new_host_config)?;
    fs::write(&config.ssh_config_path, updated_content)?;

    let output = std::process::Command::new("git")
        .args(["config", "--global", "user.name", &profile.username])
        .output()?;
    if !output.status.success() {
        return Err(GhpError::ConfigParse("Failed to set git username".to_string()));
    }

    let output = std::process::Command::new("git")
        .args(["config", "--global", "user.email", &profile.email])
        .output()?;
    if !output.status.success() {
        return Err(GhpError::ConfigParse("Failed to set git email".to_string()));
    }

    println!("Switched to profile '{}'", profile_name);
    Ok(())
}

fn update_github_host_in_ssh_config(content: &str, new_host_config: &str) -> Result<String> {
    let mut lines: Vec<&str> = content.lines().collect();
    let mut start_idx = None;
    let mut end_idx = None;
    let mut in_github_host = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("Host github.com") {
            start_idx = Some(i);
            in_github_host = true;
        } else if in_github_host {
            if trimmed.starts_with("Host ") || i == lines.len() - 1 {
                end_idx = Some(if i == lines.len() - 1 { i + 1 } else { i });
                break;
            }
        }
    }

    let result = match (start_idx, end_idx) {
        (Some(start), Some(end)) => {
            let mut new_content = String::new();
            new_content.push_str(&lines[..start].join("\n"));
            if !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(new_host_config);
            if end < lines.len() {
                new_content.push('\n');
                new_content.push_str(&lines[end..].join("\n"));
            }
            new_content
        },
        _ => {
            let mut new_content = content.to_string();
            if !new_content.is_empty() && !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(new_host_config);
            new_content
        }
    };

    Ok(result)
}

fn remove_profile(matches: &ArgMatches) -> Result<()> {
    let profile_name = matches.get_one::<String>("profile")
        .ok_or_else(|| GhpError::MissingConfig("Profile name required".to_string()))?;
    let mut config = Config::load()?;

    if config.profiles.remove(profile_name).is_some() {
        config.save()?;
        println!("Profile '{}' removed successfully!", profile_name);
        Ok(())
    } else {
        Err(GhpError::ProfileNotFound(profile_name.clone()))
    }
}