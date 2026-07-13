pub fn parse_git_output(output: &str) -> Vec<&str> {
    output.lines().filter(|l| !l.is_empty()).collect()
}

pub fn extract_branch_name(status_line: &str) -> Option<&str> {
    status_line.strip_prefix("## ").map(|rest| {
        rest.split("...")
            .next()
            .unwrap_or(rest)
            .split(' ')
            .next()
            .unwrap_or(rest)
    })
}