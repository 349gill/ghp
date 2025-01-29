# ghp
ghp is a Git configuration management tool aimed for managing several GitHub profiles and SSH/GPG keys

## Get started
For Unix and WSL systems, use
```bash
cargo install --git https://github.com/349gill/ghp.git
```
The tool does not work on Windows for now

## Troubleshooting
If you cannot access the tool directly from the terminal, try manually adding it as a PATH variable
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Usage
It is strongly recommended to use
```bash
ghp setup -s ~/.ssh/config -g ~/.ghp
```
Before doing anything.

A ghp Profile consists of a name, email, and a path to an SSH key.
The name and email should match with the user.name and user.email of the git configuration associated with the profile.

New Profiles can be created as
```
ghp add my-profile
```

Profiles can be deleted
```
ghp delete my-profile
```

To activate a specific Profile, use
```
ghp switch my-profile
```
 

