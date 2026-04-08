//! Project initialization command

use crate::cli::InitOpts;
use crate::config::Config;
use crate::templates::{ProjectData, TemplateRenderer};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(opts: InitOpts) -> Result<()> {
    let project_name = &opts.name;
    let project_dir = opts
        .dir
        .map(|d| Path::new(&d).to_path_buf())
        .unwrap_or_else(|| Path::new(project_name).to_path_buf());

    // Parse platforms
    let platforms: Vec<&str> = opts.platforms.split(',').map(|s| s.trim()).collect();
    for p in &platforms {
        if !["linux", "macos", "windows"].contains(p) {
            bail!("Unknown platform: {}. Valid options: linux, macos, windows", p);
        }
    }

    println!(
        "{} {} {}",
        "Creating".green().bold(),
        "RNL project".cyan(),
        project_name.white().bold()
    );

    // Check if directory exists
    if project_dir.exists() {
        bail!(
            "Directory '{}' already exists. Use --dir to specify a different location.",
            project_dir.display()
        );
    }

    // Create project structure
    create_project_structure(&project_dir, project_name, &platforms)?;

    println!();
    println!("{}", "✓ Project created successfully!".green().bold());
    println!();
    println!("  {}", "Next steps:".cyan());
    println!("    cd {}", project_dir.display());
    println!("    npm install");
    println!("    rnl build");
    println!("    rnl run");
    println!();

    Ok(())
}

fn create_project_structure(project_dir: &Path, name: &str, platforms: &[&str]) -> Result<()> {
    // Create directories
    let dirs = [
        "",
        "src",
        "core",
        "core/src",
        "core/include",
    ];

    for dir in &dirs {
        fs::create_dir_all(project_dir.join(dir))?;
    }

    // Create platform directories for enabled platforms
    for platform in platforms {
        let platform_dir = match *platform {
            "linux" => "platforms/linux/src/elements",
            "macos" => "platforms/macos/Sources/Elements",
            "windows" => "platforms/windows/src",
            _ => continue,
        };
        fs::create_dir_all(project_dir.join(platform_dir))?;
    }

    // Generate project data
    let data = ProjectData::new(name, platforms);

    // Create rnl.toml
    let config = Config::default_for_project(name, platforms);
    config.save(project_dir)?;
    println!("  {} rnl.toml", "created".green());

    // Create package.json
    let package_json = generate_package_json(&data);
    fs::write(project_dir.join("package.json"), package_json)?;
    println!("  {} package.json", "created".green());

    // Create tsconfig.json
    let tsconfig = generate_tsconfig();
    fs::write(project_dir.join("tsconfig.json"), tsconfig)?;
    println!("  {} tsconfig.json", "created".green());

    // Create src/index.tsx
    let index_tsx = generate_index_tsx();
    fs::write(project_dir.join("src/index.tsx"), index_tsx)?;
    println!("  {} src/index.tsx", "created".green());

    // Create src/App.tsx
    let app_tsx = generate_app_tsx(name);
    fs::write(project_dir.join("src/App.tsx"), app_tsx)?;
    println!("  {} src/App.tsx", "created".green());

    // Create .gitignore
    let gitignore = generate_gitignore();
    fs::write(project_dir.join(".gitignore"), gitignore)?;
    println!("  {} .gitignore", "created".green());

    // Create README.md
    let readme = generate_readme(name);
    fs::write(project_dir.join("README.md"), readme)?;
    println!("  {} README.md", "created".green());

    Ok(())
}

fn generate_package_json(data: &ProjectData) -> String {
    format!(
        r#"{{
  "name": "{}",
  "version": "{}",
  "description": "{}",
  "main": "src/index.tsx",
  "scripts": {{
    "build": "rnl build",
    "start": "rnl run",
    "typecheck": "tsc --noEmit"
  }},
  "dependencies": {{
    "react": "^18.2.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.2.0",
    "typescript": "^5.3.0",
    "esbuild": "^0.20.0"
  }}
}}
"#,
        data.name, data.version, data.description
    )
}

fn generate_tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "types": ["rnl"]
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "target", "dist"]
}
"#
    .to_string()
}

fn generate_index_tsx() -> String {
    r#"import { render } from 'rnl';
import { App } from './App';

render(<App />);
"#
    .to_string()
}

fn generate_app_tsx(name: &str) -> String {
    format!(
        r#"import {{ useState }} from 'rnl';

export function App() {{
    const [count, setCount] = useState(0);

    return (
        <box orientation="vertical" spacing={{12}} style={{{{ padding: 24 }}}}>
            <text>Welcome to {}!</text>
            
            <box orientation="horizontal" spacing={{8}}>
                <button label="-" onClick={{() => setCount(c => c - 1)}} />
                <text>{{String(count)}}</text>
                <button label="+" onClick={{() => setCount(c => c + 1)}} />
            </box>
            
            <button 
                label="Reset" 
                onClick={{() => setCount(0)}}
                enabled={{count !== 0}}
            />
        </box>
    );
}}
"#,
        name
    )
}

fn generate_gitignore() -> String {
    r#"# Build artifacts
/target
/dist
/node_modules

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Debug
*.log
"#
    .to_string()
}

fn generate_readme(name: &str) -> String {
    format!(
        r#"# {}

An application built with [RNL](https://github.com/neutrino2211/rnl) (React Native Libre).

## Getting Started

```bash
# Install dependencies
npm install

# Build the application
rnl build

# Run the application
rnl run
```

## Project Structure

```
{}/
├── src/              # React/TypeScript source code
│   ├── index.tsx     # Entry point
│   └── App.tsx       # Main app component
├── core/             # Rust core (usually not modified)
├── platforms/        # Platform-specific implementations
├── rnl.toml          # Project configuration
└── package.json      # Node.js dependencies
```

## Building for Release

```bash
rnl build --release
```

## Learn More

- [RNL Documentation](https://rnl.dev/docs)
- [React Documentation](https://react.dev)
"#,
        name, name
    )
}
