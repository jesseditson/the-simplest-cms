use archival;
use std::error::Error;

fn get_args(args: Vec<&str>) -> impl Iterator<Item = String> {
    let mut a = vec!["archival".to_string()];
    for arg in args {
        a.push(arg.to_string())
    }
    a.into_iter()
}

#[test]
fn build_basics() -> Result<(), Box<dyn Error>> {
    archival::binary(get_args(vec!["build", "tests/fixtures/website"]))?;
    Ok(())
}
