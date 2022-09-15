use clap::Parser;

/// Minecraft Operator
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    mode: Mode,
}

#[derive(clap::Subcommand, Debug)]
enum Mode {
    Run,
    CrdGen,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.mode {
        Mode::Run => minecraft_operator::operator::run().expect("failed to run operator"),
        Mode::CrdGen => minecraft_operator::crd::generate_crds().expect("failed to generate crds"),
    };

    Ok(())
}
