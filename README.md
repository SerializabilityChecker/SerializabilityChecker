
This README file is organized as follows:

- In part (i), we explain in detail the different files and
directories of the artifact, including the code architecture for our tool.

- In part (ii), we explain how to replay the experiments reported in
our paper. Specifically, we explain how to run it on singular examples with various optimization combinations,
as well as how to run the suite of examples to generate the result in our paper.

- In part (iii), we provide an overview of our raw experimental results, and the code for generating the 
plots and the tables in our paper.

We note that we intend to make the whole artifact, including the
original input programs, and the raw experimental results, permanently available
online, along with the final version of this paper.


########################################################
(i) Different files and directories of the artifact
########################################################

Below is a summary of the contents of the artifact directory (besides the license and this README), in alphabetical order:


(*) build.rs

A Rust build script that runs before compilation to generate FFI bindings via bindgen and compile any bundled C code using the cc crate, ensuring the ser library can interface with native components.

(*) Cargo.toml

The Cargo manifest for the ser package, specifying crate metadata (name, version, edition), its runtime dependencies (e.g., serde, csv, lazy_static), and build-time dependencies (bindgen, cc).

(*) examples

A directory with our experiments is divided into a <json> directory or a <ser> directory.

(*) raw_experimental_results

A directory with our experimental recordings for the tables and plots in the paper. The original executions are summarized in the JSONL files in each subdirectory.

(*) scripts

- (scripts/analyze_examples.py) Runs optimized serializability analyses over all SER (and JSON) examples in parallel, capturing CPU times, result statuses (serializable/not), and generates a Markdown report with per‐example and summary statistics
- (scripts/check_proofs.py) Executes the serializability checker on each JSON example, inspects proof certificate validity and counterexample traces, flags problematic cases, and outputs a detailed JSON summary of results
- (scripts/generate_big_summary_table_from_jsonl.py) Reads the JSONL serializability statistics, writes a CSV summary, and produces a LaTeX “big table” summarizing per‐benchmark results with symbols for serializability, non-serializability, and timeouts
- (scripts/generate_cactus_plot_and_render_stats.py) Loads serializability stats, generates multiple LaTeX tables (comprehensive, pruning effectiveness, timing comparison, optimization breakdown, summary stats), and optionally creates a cactus plot comparing solver performance across optimization configurations
- (scripts/generate_petri_reduction_graph_from_jsonl.py) Filters the JSONL stats for runs with all optimizations enabled, computes Petri net place/transition reductions per example, and saves a PDF plot visualizing size‐reduction percentages
- (scripts/generate_semilinear_reduction_table_from_jsonl.py) Loads and filters JSONL statistics to identify common examples across optimization scenarios, computes semilinear‐set component counts and period statistics, and outputs a LaTeX table comparing mean/max values
- (scripts/generate_stats_table_for_big_table.py) Reads the CSV summary of serializability runs, computes average and median times for certificate generation, validation, and total analysis (grouped by serializable vs. non-serializable), and writes a LaTeX stats table


(*) ser_lang_vscode

A directory with scripts for VS-Code visualizations

(*) smpt_wrapper.sh 

A Bash wrapper that activates the SMPT Python virtual environment and invokes the SMPT model checker on given inputs


(*) src

- (src/old/affine_constraints.rs) Legacy code for parsing and representing affine constraints in Presburger sets, superseded by newer modules but kept for reference and regression tests
- (src/debug_report.rs) Generates detailed human-readable reports of intermediate data (e.g., sets, proofs) to aid debugging and trace algorithmic steps
- (src/deterministic_map.rs) Implements a map with a stable, deterministic iteration order to ensure reproducible behavior across runs
- (src/expr_to_ns.rs) Translates parsed logical expressions into the internal “Normal Set” (NS) representation, bridging AST nodes to arithmetic sets
- (src/graphviz.rs) Provides helper functions to emit Graphviz/DOT descriptions for visualizing state-space graphs and Petri net structures
- (src/isl.rs) Rust FFI bindings and wrapper routines for the ISL (Integer Set Library), enabling the creation and manipulation of semilinear sets via C
- (src/isl_helpers.c) C helper implementations supporting the ISL FFI (e.g., custom allocation or auxiliary routines) compiled by the build script
- (src/isl_wrapper.h) Header declaring the C API exposed to Rust for ISL operations, included by isl_helpers.c and consumed via bindgen
- (src/kleene.rs) Defines Kleene-algebraic operations (union, concatenation, closure) and fixpoint computations used in reachability analyses
- (src/main.rs) CLI entry point: parses command-line arguments, selects subcommands (e.g., reachability, conversion), and orchestrates high-level workflow
- (src/ns.rs) Core definitions for “Normal Sets” (NS): algebraic data types representing Presburger-definable sets and basic constructors
- (src/ns_decision.rs) Implements decision procedures on NS values (e.g., emptiness checks, membership queries) using automata-theoretic or algebraic methods
- (src/ns_to_petri.rs) Translates NS representations into Petri net models, enabling the use of Petri-net-based reachability tools on arithmetic sets
- (src/parser.rs) Text parser for the DSL or input formats (SMT-like or custom), producing ASTs for constraints, transitions, and set definitions
- (src/petri.rs) Defines Petri net data structures (places, transitions) and basic operations for building and analyzing nets derived from NS
- (src/presburger.rs) High-level API for Presburger arithmetic: conjunction, disjunction, projection, elimination, and other logical operations on NS
- (src/presburger_harmonize_tests.rs) Test suite ensuring that multiple Presburger sets can be harmonized (aligned on shared variables) correctly before combination
- (src/proof_parser.rs) Parses serialized proof artifacts (e.g., invariants and derivation steps) emitted by the proof-aware reachability engine
- (src/proofinvariant_to_presburger.rs) Converts proof invariants back into Presburger-set form, linking proof certificates with arithmetic representations
- (src/reachability.rs) Core algorithm for computing reachability over Presburger sets under given transitions, without proof generation
- (src/reachability_with_proofs.rs) Extended reachability engine that also constructs formal proof objects (invariants, derivations) alongside reachable sets
- (src/semilinear.rs) Implements semilinear sets (finite unions of linear sets) and operations (union, intersection, projection) on them
- (src/size_logger.rs) Records and logs the sizes of key data structures (number of states, constraints, components) to CSV for performance analysis
- (src/smpt.rs) Integrates with an SMT solver for testing or verifying properties of Presburger formulas, acting as a proof oracle
- (src/spresburger.rs) Specialized “streaming” or “symbolic” Presburger engine offering optimized operations for large or sparse constraint sets
- (src/stats.rs) Gathers runtime and memory statistics across various modules, formatting summaries for console or log output
- (src/utils.rs) Miscellaneous utilities and helper functions (string handling, common errors, small math routines) shared across the codebase 


########################################################
(ii) Using the tool and running the experiments
########################################################

In this part, we will explain the 3 steps required to run (existing and new)
experiments described in our paper:

    1. Installing SMPT and the virtual environment
    2. Installing dependencies (if on MacOS)
    3. Running the experiments

We note that for ease of explanation, <> brackets were
added in the below description of files.

Please ensure that your Bash and Python scripts are executable before you run them.

Part 1: Installing SMPT and the virtual environment

Got to the SMPT repository and clone it - https://github.com/nicolasAmat/SMPT

Dependencies of SMPT.

```bash
# create a virtual environment:
cd ser/SMPT
python3 -m venv myenv

# Once activating the virtual environment:
# install Z3 (and add the Z3 binary to your PATH variable)
pip install z3-solver

# install sexpdata
pip install sexpdata
```
   
For Z3 you can also download, extract, and add to the PATH an official release (compatible with your libc version): https://github.com/Z3Prover/z3/releases (however, if the PIP package works well, there is no need to do this manually).

After cloning SMPT and installing the dependencies, please update the hard-coded path in <smpt_wrapper.sh> to match the location of your local <SMPT> directory.


Part 2: Installing additional dependencies:

## Dependencies

Depends on [isl](https://libisl.sourceforge.io/), which you may already have
installed (it comes with GCC).  For a non-standard install, you may need to set
the `ISL_PREFIX` environment variable.


### macOS Setup

On macOS, you'll need to install ISL and some build tools. Here's a step-by-step guide:

```bash
# Install ISL using Homebrew
brew install isl

# Install automake (needed by the isl-rs crate)
brew install automake

# Set the ISL_PREFIX environment variable (add this to your ~/.zshrc or ~/.bashrc)
export ISL_PREFIX=/opt/homebrew/Cellar/isl/0.27
```

If you're having issues with the ISL path, verify the installed version with `brew info isl` and adjust the path accordingly.

## VSCode Integration

This repository includes VSCode configuration for syntax highlighting of `.ser` files in the `ser-lang-vscode` directory. 

Features:
- Syntax highlighting
- Auto-closing of brackets and parentheses
- Code folding

Install with:

```bash
cd ser-lang-vscode
./build-vsix.sh
```

Note - the script will automatically build and install the extension. You may need to restart VSCode to see the changes.


Part 3 - Running our tool:

NOTE - all the scripts are run from the root <ser> directory

***** Case 3(a): running a single existing example *****

- These examples are in <ser/examples/json/benchmark_name.json> or <ser/examples/ser/benchmark_name.ser>

- In order to run a single example execute the following command:


```
cargo run <PATH_TO_SER_FILE>  [--timeout seconds] [--without-bidirectional] [--without-remove-redundant] [--without-generate-less] [--without-smart-kleene-order]
```

```
Options:

[--timeout seconds] -> a timeout threshold (in seconds)
[--without-bidirectional] -> turn OFF the bidirectional pruning optimization (default: ON) 
[--without-remove-redundant] -> turn OFF the removal of redundant constraints optimization (default: ON) 
[--without-generate-less] -> turn OFF the generation of less constraints optimization (default: ON) 
[--without-smart-kleene-order] -> turn OFF the strategic Kleene order optimization (default: ON) 
```

The result of running a single example <FILE_NAME> is:

(*) appending to the <out/serializability_stats.jsonl> file:

(***) this is a single <json> record of the last run. The record will summarize the time breakdown, as well as the optimizations used, the number of disjuncts, the size of the PN, the size of the semilinear set, and the results of the query.



(*) generating a subdirectory <ser/out/FILE_NAME> which includes:

- <FILE_NAME> --- the original input program (JSON/SER file)
- <network.svg> --- an image of the NS and serialized NFA (SVG file), and a <.dot> file
- <petri.svg> --- an image of the PN (SVG file), and a <.dot> file
- <semilinear.txt> --- a text file encoding the language (and regex) of the NFA representing the serializable executions
- <petri.net> --- a text file encoding the PN without the requests, and prior to pruning (which we run per each disjunct)
- <petri_with_requests.net> ---- a text file encoding the PN with requests, and prior to pruning (which we run per each disjunct)
- <smpt_petri_disjunct_i.net> --- the pruned PN corresponding to disjunct #i
- <smpt_constraints_disjunct_i.xml> --- an XML file encoding the reachability query for disjunct #i
- <smpt_constraints_disjunct_i.stdout> --- SMPT's outputs when running the query for disjunct #i
- <smpt_constraints_disjunct_i.stderr> --- SMPT's errors (if they occur) when running the query for disjunct #i
- <smpt_constraints_disjunct_i_proof.txt> --- SMPT's proof if the query is UNSAT (note that this file is not always generated).


***** Case 3(b): running all existing examples simultaneously *****

This can be done by running the following script:

``` 
analyze_examples.py [-h] [--timeout TIMEOUT] [--jobs JOBS] [--use-cache] [--without-remove-redundant]
                           [--without-generate-less] [--without-smart-kleene-order] [--without-bidirectional]
                           [--no-viz] [--path PATH] [--all-optimizations] [--no-optimizations]
                           [--optimization-comparison] [--full-optimization-study]
```

This script analyzes SER/JSON examples with optional cargo flags (optimized only). It allows running in parallel as well as comparing optimization combinations.

``` 
Options:
  -h, --help            show this help message and exit
  --timeout TIMEOUT     Timeout seconds
  --jobs JOBS           Parallel jobs
  --use-cache           Enable caching
  --without-remove-redundant
                        Disable redundant removal
  --without-generate-less
                        Disable generate-less optimization
  --without-smart-kleene-order
                        Disable smart Kleene ordering (a.k.a. ``strategic'' Kleene ordering)
  --without-bidirectional
                        Disable bidirectional optimization
  --no-viz              Disable visualization generation
  --path PATH           Specific file or directory to analyze
  --all-optimizations   Run with all optimizations enabled (default)
  --no-optimizations    Run with all optimizations disabled
  --optimization-comparison
                        Run twice: once with all optimizations, once without
  --full-optimization-study
                        Run 6 times: no opts, all opts, and each individual opt
```

Running the script will:

(*) generate the relevant subdirectories in <ser/out>

(*) append new JSON records to the <out/serializability_stats.jsonl> file. 



***** Case 3(c): running new examples *****

The user can also generate novel examples and run the tool on them. These can be in one of two formats: the NS (JSON) format or the serializable (SER) format. We elaborate on both.

## TYPE 1 INPUT: Network System (json files in ser/examples/json)

Example for a <.json> file:

    {
        "requests": [["Req1", "L0"], ["Req2", "L1"], ["Req3", "L2"]],
        "responses": [["L0", "RespA"], ["L1", "RespB"], ["L2", "RespC"]],
        "transitions": [
            ["L0", "G0", "L1", "G1"],
            ["L1", "G1", "L2", "G2"],
            ["L2", "G2", "L0", "G3"]
        ]
    }

##  TYPE 1 INPUT: Syntax (ser files in ser/examples/ser)

### Expression syntax for a <.ser> file:

e ::=
  | n                     (constant) 
  | x := e                (local variable / packet field write)
  | x                     (read)
  | X := e                (global variable / switch variable)
  | X                     (read)
  | e + e                 (addition)
  | e - e                 (subtraction)
  | e == e                (equality check)
  | e ; e                 (sequence)
  | if(e){e}else{e}       (conditional)
  | while(e){e}           (loop)
  | yield                 (yields to the scheduler; allows other threads/packets to run)
  | ?                     (nondeterministic choice between 0 and 1)
  | // text                (single-line comment, ignored by the parser)

### Multiple Requests Syntax

The parser supports multiple top-level programs with named requests:

```
request <request_name> {
    // program body
}

request <another_request_name> {
    // another program body
}
```

Examples:

```
request login {
  x := 1;
  yield;
  r := 42
}

request logout {
  y := 2;
  yield;
  r := 10
}
```

Example with arithmetic operations and comments:

```
// Main request with arithmetic operations
request main {
  x := 5 + 3;     // x = 8
  y := x - 2;     // y = 6
  z := y + y;     // z = 12
  
  // This yield allows other threads/packets to run
  yield
}
```

Once the file encoding the example is generated, the user can run it similar to steps 3(a) or 3(b) mentioned above.

########################################################
(iii) Information regarding the raw experimental results
########################################################

The <raw_experimental_results> directory in the root includes the experiments reported in the paper, along with the accompanying plots and tables.




-----------------------------------------------------------------------------------------------------------------


Thank you for your time!

The authors



