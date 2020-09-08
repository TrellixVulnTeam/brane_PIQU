use crate::packages;
use anyhow::{Context, Result};
use console::style;
use specifications::common::Function;
use specifications::container::ContainerInfo;
use specifications::package::PackageInfo;
use std::fmt::Write as FmtWrite;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

type Map<T> = std::collections::HashMap<String, T>;

const INIT_URL: &str = concat!(
    "https://github.com/onnovalkering/brane/releases/download/",
    concat!("v", env!("CARGO_PKG_VERSION")),
    "/brane-init"
);

///
///
///
pub fn handle(
    context: PathBuf,
    file: PathBuf,
    init_path: Option<PathBuf>,
) -> Result<()> {
    let context = fs::canonicalize(context)?;
    debug!("Using {:?} as build context", context);

    let ecu_file = context.join(file);
    let ecu_reader = BufReader::new(File::open(&ecu_file)?);
    let ecu_document = ContainerInfo::from_reader(ecu_reader)?;

    // Prepare package directory
    let dockerfile = generate_dockerfile(&ecu_document, init_path.is_some())?;
    let package_info = generate_package_info(&ecu_document)?;
    let package_dir = packages::get_package_dir(&package_info.name, Some(&package_info.version))?;
    prepare_directory(&ecu_document, dockerfile, init_path, &context, &package_info, &package_dir)?;

    debug!("Successfully prepared package directory.");

    // Build ECU image
    let tag = format!("{}:{}", ecu_document.name, ecu_document.version);
    build_ecu_image(&package_dir, tag)?;

    println!(
        "Successfully built version {} of ECU package {}.",
        style(&package_info.version).bold().cyan(),
        style(&package_info.name).bold().cyan(),
    );

    Ok(())
}

///
///
///
fn generate_package_info(container_info: &ContainerInfo) -> Result<PackageInfo> {
    // Construct function descriptions
    let mut functions = Map::<Function>::new();
    for (action_name, action) in &container_info.actions {
        let arguments = action.input.clone();
        let pattern = action.pattern.clone();
        let return_type = action.output[0].data_type.to_string();

        let function = Function::new(arguments, pattern, return_type);
        functions.insert(action_name.clone(), function);
    }

    // Create and write a package.yml file.
    let package_info = PackageInfo::new(
        container_info.name.clone(),
        container_info.version.clone(),
        container_info.description.clone(),
        String::from("ecu"),
        Some(functions),
        None,
    );

    Ok(package_info)
}

///
///
///
fn generate_dockerfile(
    ecu_document: &ContainerInfo,
    override_init: bool,
) -> Result<String> {
    let mut contents = String::new();
    let base = ecu_document
        .base
        .clone()
        .unwrap_or_else(|| String::from("ubuntu:20.04"));

    // Add default heading
    writeln!(contents, "# Generated by Brane")?;
    writeln!(contents, "FROM {}", base)?;

    // Add environemt variables
    if let Some(environment) = &ecu_document.environment {
        for (key, value) in environment {
            writeln!(contents, "ENV {}={}", key, value)?;
        }
    }

    // Add dependencies
    if base.starts_with("alpine") {
        write!(contents, "RUN apk add --no-cache ")?;
    } else {
        write!(contents, "RUN apt-get update && apt-get install -y ")?;
    }
    if let Some(dependencies) = &ecu_document.dependencies {
        for dependency in dependencies {
            write!(contents, "{} ", dependency)?;
        }
    }
    writeln!(contents)?;

    // Add default init library
    if override_init {
        writeln!(contents, "ADD init init")?;
    } else {
        writeln!(contents, "ADD {} init", INIT_URL)?;
        writeln!(contents, "RUN chmod +x init")?;
    }

    // Copy files
    writeln!(contents, "COPY container.yml /container.yml")?;
    writeln!(contents, "ADD wd.tar.gz /opt")?;
    writeln!(contents, "WORKDIR /opt/wd")?;

    // Add installation
    if let Some(install) = &ecu_document.install {
        for line in install {
            writeln!(contents, "RUN {}", line)?;
        }
    }

    writeln!(contents, "WORKDIR /")?;
    writeln!(contents, "ENTRYPOINT [\"./init\"]")?;

    Ok(contents)
}

///
///
///
fn prepare_directory(
    ecu_document: &ContainerInfo,
    dockerfile: String,
    init_path: Option<PathBuf>,
    context: &PathBuf,
    package_info: &PackageInfo,
    package_dir: &PathBuf,
) -> Result<()> {
    fs::create_dir_all(&package_dir)?;
    debug!("Created {:?} as package directory", package_dir);

    // Write container.yml to package directory.
    let mut buffer = File::create(&package_dir.join("container.yml"))?;
    write!(buffer, "{}", serde_yaml::to_string(&ecu_document)?)?;

    // Write Dockerfile to package directory
    let mut buffer = File::create(package_dir.join("Dockerfile"))?;
    write!(buffer, "{}", dockerfile)?;

    // Write Dockerfile to package directory
    let mut buffer = File::create(package_dir.join("package.yml"))?;
    write!(buffer, "{}", serde_yaml::to_string(&package_info)?)?;

    // Copy custom init binary to package directory
    if let Some(init_path) = init_path {
        fs::copy(fs::canonicalize(init_path)?, package_dir.join("init"))?;
    }

    // Create the working directory and copy required files.
    let wd = package_dir.join("wd");
    if let Some(files) = &ecu_document.files {
        for file in files {
            let wd_path = wd.join(file);
            if let Some(parent) = wd_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(&parent)?;
                }
            }

            let file_path = context.join(file);
            fs::copy(&file_path, &wd_path)
                .with_context(|| format!("Couldn't find {:?} within the build context.", file_path))?;
                
            debug!("Copied {:?} to working directory", file_path);
        }
    }

    // Archive the working directory and remove the original.
    let output = Command::new("tar")
        .arg("-zcf")
        .arg("wd.tar.gz")
        .arg("wd")
        .current_dir(&package_dir)
        .output()
        .expect("Couldn't run 'tar' command.");

    if !output.status.success() {
        return Err(anyhow!("Failed to prepare working directory archive."));
    }

    let output = Command::new("rm")
        .arg("-rf")
        .arg("wd")
        .current_dir(&package_dir)
        .output()
        .expect("Couldn't run 'rm' command.");

    if !output.status.success() {
        warn!("Failed to cleanup working directory.");
    }

    Ok(())
}

///
///
///
fn build_ecu_image(
    package_dir: &PathBuf,
    tag: String,
) -> Result<()> {
    let buildx = Command::new("docker")
        .arg("buildx")
        .output()
        .expect("Couldn't run 'docker' command.");

    if !buildx.status.success() {
        return Err(anyhow!("Failed to build ECU image. Is BuildKit enabled (see documentation)?"));
    }

    let output = Command::new("docker")
        .arg("buildx")
        .arg("build")
        .arg("--output")
        .arg("type=docker,dest=image.tar")
        .arg("--tag")
        .arg(tag)
        .arg(".")
        .current_dir(&package_dir)
        .status()
        .expect("Couldn't run 'docker' command.");

    if !output.success() {
        return Err(anyhow!("Failed to build ECU image. See Docker output above for more information."));
    }

    Ok(())
}
