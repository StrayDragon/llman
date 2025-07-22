use anyhow::Result;
use clap::Args;
use ignore::WalkBuilder;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct TreeArgs {
    /// The directory to scan, defaults to the current directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output the result to a file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Do not respect .gitignore files
    #[arg(long)]
    pub no_ignore: bool,

    /// Set the maximum depth to traverse
    #[arg(short, long, default_value = "2")]
    pub max_depth: Option<usize>,

    /// Intelligently include content of meaningful files
    #[arg(long)]
    pub append_default_context: bool,
}

pub fn run(args: &TreeArgs) -> Result<()> {
    let path = &args.path;
    let mut output_buffer = String::new();

    let meaningful_files = if args.append_default_context {
        vec![
            "Cargo.toml",
            "README.md",
            "package.json",
            "pyproject.toml",
            "go.mod",
        ]
    } else {
        vec![]
    };
    let mut meaningful_content = String::new();

    let mut walker = WalkBuilder::new(path);
    walker.hidden(!args.no_ignore);
    walker.parents(!args.no_ignore);
    walker.git_ignore(!args.no_ignore);
    walker.git_global(!args.no_ignore);
    walker.git_exclude(!args.no_ignore);

    if let Some(max_depth) = args.max_depth {
        walker.max_depth(Some(max_depth));
    }

    // 收集所有条目
    let mut entries = Vec::new();
    for result in walker.build() {
        let entry = result?;
        entries.push(entry);
    }

    // 按路径排序以确保一致的输出
    entries.sort_by(|a, b| a.path().cmp(b.path()));

    // 构建树形结构
    let mut path_to_children: std::collections::HashMap<PathBuf, Vec<&ignore::DirEntry>> =
        std::collections::HashMap::new();

    for entry in &entries {
        if let Some(parent) = entry.path().parent() {
            path_to_children
                .entry(parent.to_path_buf())
                .or_default()
                .push(entry);
        }
    }

    // 递归函数生成树形输出
    fn generate_tree_output(
        current_path: &std::path::Path,
        prefix: &str,
        path_to_children: &std::collections::HashMap<PathBuf, Vec<&ignore::DirEntry>>,
        output: &mut String,
        meaningful_files: &[&str],
        meaningful_content: &mut String,
        append_context: bool,
    ) -> Result<()> {
        if let Some(children) = path_to_children.get(current_path) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| {
                // 目录优先，然后按名称排序
                let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(b.file_name()),
                }
            });

            for (i, entry) in sorted_children.iter().enumerate() {
                let is_last = i == sorted_children.len() - 1;
                let file_name = entry.file_name().to_string_lossy();
                let is_dir = entry.file_type().unwrap().is_dir();

                // 为目录添加 / 后缀以区分文件和目录
                let display_name = if is_dir {
                    format!("{}/", file_name)
                } else {
                    file_name.to_string()
                };

                let tree_symbol = if is_last { "└── " } else { "├── " };
                output.push_str(&format!("{}{}{}\n", prefix, tree_symbol, display_name));

                // 检查是否需要添加文件内容
                if append_context && !is_dir && meaningful_files.contains(&file_name.as_ref()) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        meaningful_content.push_str(&format!(
                            "\n\n---\n\n# {}\n\n```\n{}\n```",
                            entry.path().display(),
                            content
                        ));
                    }
                }

                // 递归处理子目录
                if is_dir {
                    let new_prefix = if is_last {
                        format!("{}    ", prefix)
                    } else {
                        format!("{}│   ", prefix)
                    };
                    generate_tree_output(
                        entry.path(),
                        &new_prefix,
                        path_to_children,
                        output,
                        meaningful_files,
                        meaningful_content,
                        append_context,
                    )?;
                }
            }
        }
        Ok(())
    }

    // 生成根目录的输出
    if let Some(file_name) = path.file_name() {
        output_buffer.push_str(&format!("{}\n", file_name.to_string_lossy()));
    } else {
        output_buffer.push_str(".\n");
    }

    generate_tree_output(
        path,
        "",
        &path_to_children,
        &mut output_buffer,
        &meaningful_files,
        &mut meaningful_content,
        args.append_default_context,
    )?;

    let final_output = if meaningful_content.is_empty() {
        output_buffer.trim_end().to_string()
    } else {
        format!("{}\n{}", output_buffer.trim_end(), meaningful_content)
    };

    if let Some(output_file) = &args.output {
        fs::write(output_file, &final_output)?;
        println!("Output written to {}", output_file.display());
    } else {
        println!("{}", final_output);
    }

    Ok(())
}
