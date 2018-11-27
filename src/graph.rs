use std::fmt::Write as FmtWrite;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

use conllx::graph::{Node, Sentence};
use conllx::token::Features;
use failure::{Error, ResultExt};
use itertools::Itertools;

pub trait Dot {
    fn dot(&self) -> Result<String, Error>;
}

impl Dot for Sentence {
    fn dot(&self) -> Result<String, Error> {
        graph_to_dot(self)
    }
}

pub trait Tikz {
    fn tikz(&self) -> Result<String, Error>;
}

impl Tikz for Sentence {
    fn tikz(&self) -> Result<String, Error> {
        graph_to_tikz(self)
    }
}

pub trait Tokens {
    fn tokens(&self) -> Vec<&str>;
}

impl Tokens for Sentence {
    fn tokens(&self) -> Vec<&str> {
        let mut tokens = Vec::new();
        for token_idx in 0..self.len() {
            let token = ok_or!(self[token_idx].token(), continue);
            tokens.push(token.form());
        }

        tokens
    }
}

pub trait Svg {
    fn svg(&self) -> Result<String, Error>;
}

impl Svg for Sentence {
    fn svg(&self) -> Result<String, Error> {
        let dot = self.dot()?;
        dot_to_svg(&dot)
    }
}

fn dot_to_svg(dot: &str) -> Result<String, Error> {
    // FIXME: bind against C library?

    // Spawn Graphviz dot for rendering SVG (Fixme: bind against C library?).
    let process = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Could not start Graphviz dot")?;

    process
        .stdin
        .unwrap()
        .write_all(dot.as_bytes())
        .context("Could not write graph to dot stdin")?;

    let mut svg = String::new();
    process
        .stdout
        .unwrap()
        .read_to_string(&mut svg)
        .context("Could not read graph SVG from dot stdout")?;

    Ok(svg)
}

fn escape_str<S>(s: S) -> String
where
    S: AsRef<str>,
{
    s.as_ref().replace('"', r#"\""#)
}

fn graph_to_dot(sentence: &Sentence) -> Result<String, Error> {
    let mut dot = String::new();

    dot.push_str("digraph deptree {\n");
    dot.push_str("graph [charset = \"UTF-8\"]\n");
    dot.push_str(
        "node [shape=plaintext, height=0, width=0, fontsize=12, fontname=\"Helvetica\"]\n",
    );

    for token_idx in 0..sentence.len() {
        let token = ok_or!(sentence[token_idx]
            .token(), continue);

        let marked = 
            token.features()
            .map(Features::as_map)
            .map(|m| m.contains_key("mark"))
            .unwrap_or(false);

        if marked {
            writeln!(
                &mut dot,
                r#"n{}[label="{}", fontcolor="firebrick3"];"#,
                token_idx,
                escape_str(token.form())
            )?;
        } else {
            writeln!(
                &mut dot,
                r#"n{}[label="{}"];"#,
                token_idx,
                escape_str(token.form())
            )?;
        }
    }

    dot.push_str("edge [color=\"#4b0082\", fontsize=\"8\", fontname=\"Courier New\"]\n");

    let graph = sentence.graph();
    for token_idx in 0..sentence.len() {
        let triple = ok_or!(graph.head(token_idx), continue);
        if sentence[triple.head()] == Node::Root {
            continue;
        }

        writeln!(
            &mut dot,
            r#"n{} -> n{}[label="{}"];"#,
            triple.head(),
            triple.dependent(),
            escape_str(triple.relation().unwrap_or("_"))
        )?;
    }

    dot.push_str("}");

    Ok(dot)
}

fn graph_to_tikz(sentence: &Sentence) -> Result<String, Error> {
    let mut dot = String::new();

    dot.push_str("\\documentclass{standalone}\n\n");
    dot.push_str("\\usepackage{tikz-dependency}\n\n");
    dot.push_str("\\begin{document}\n\n");
    dot.push_str("\\begin{dependency}\n");
    dot.push_str("\\begin{deptext}");

    dot.push_str(&(0..sentence.len())
        .filter_map(|idx| {
            let token = ok_or!(sentence[idx].token(), return None);
            let marked = token.features()
                .map(Features::as_map)
                .map(|m| m.contains_key("mark"))
                .unwrap_or(false);

            if marked {
                Some(format!("\\underline{{{}}}", token.form()))
            } else {
                Some(token.form().to_owned())
            }
        })
        .join(" \\& "));

    dot.push_str("\\\\\n\\end{deptext}\n");

    let graph = sentence.graph();
    for token_idx in 0..sentence.len() {
        let triple = ok_or!(graph.head(token_idx), continue);

        writeln!(
            &mut dot,
            "\\depedge{{{}}}{{{}}}{{{}}}",
            triple.head() + 1,
            triple.dependent() + 1,
            escape_str(triple.relation().unwrap_or("_"))
        )?;
    }

    dot.push_str("\\end{dependency}\n\n");
    dot.push_str("\\end{document}");

    Ok(dot)
}
