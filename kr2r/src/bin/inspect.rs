use clap::Parser;
use kr2r::compact_hash::HashConfig;
use kr2r::taxonomy::Taxonomy;
use kr2r::IndexOptions;
use std::io::Result;

/// inspects the contents of a Kraken 2 hash table file
#[derive(Parser, Debug, Clone)]
#[clap(version, about = "inspect")]
struct Args {
    /// The file path for the Kraken 2 index.
    #[clap(short = 'H', long = "index-filename", value_parser, required = true)]
    index_filename: String,

    /// The file path for the Kraken 2 taxonomy.
    #[clap(short = 't', long = "taxonomy-filename", value_parser)]
    taxonomy_filename: Option<String>,

    /// The file path for the Kraken 2 options.
    #[clap(short = 'o', long = "options-filename", value_parser)]
    options_filename: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(option_filename) = args.options_filename {
        let idx_opts = IndexOptions::read_index_options(option_filename)?;
        println!("index option {:?}", idx_opts);
    }
    if let Some(taxonomy_filename) = args.taxonomy_filename {
        let taxo = Taxonomy::from_file(&taxonomy_filename)?;
        println!("taxonomy node count {:?}", taxo.node_count());
    }

    let config = HashConfig::from_hash_header(args.index_filename.clone())?;

    println!("compact hash table {:?}", config);

    Ok(())
}
