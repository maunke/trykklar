use anyhow::Result;
use clap::Parser;
use std::collections::HashSet;
use trykklar::inventory::step::{ColorSpaceColorantNames, PaintedColorSpaces};
use trykklar::pdf::color::ColorantName;
use trykklar::pdf::content::ContentReference;
use trykklar::pdf::xobject::FormXObjectId;
use trykklar::pdf::{OcConfig, Operator, XObject};
use trykklar::{PageWalker, Pdf, WalkerProcessor};

#[derive(Parser)]
struct Args {
    path: String,
}

pub struct IsolatedWhiteUnderprints {
    white_color_names: Vec<ColorantName>,
    isolated_groups: HashSet<FormXObjectId>,
    inderterminates: usize,
    findings: usize,
}

impl IsolatedWhiteUnderprints {
    fn new() -> Self {
        Self {
            white_color_names: vec![
                ColorantName::Name("White".into()),
                ColorantName::Name("Deckweiß".into()),
                ColorantName::Name("Weiß".into()),
                ColorantName::Name("bila-white".into()),
            ],
            isolated_groups: HashSet::new(),
            inderterminates: 0,
            findings: 0,
        }
    }
}

impl WalkerProcessor for IsolatedWhiteUnderprints {
    fn process(&mut self, step: &trykklar::pdf::ContentWalkerStep) {
        // filter for form xobject with a group entry where isolation is true
        if let Operator::PaintXObject(xobject) = step.operator() {
            match xobject {
                Ok(XObject::Form(form)) => {
                    let Some(Ok(group)) = form.group() else {
                        return;
                    };
                    match group.isolation() {
                        Ok(isolation) => {
                            if isolation.get() {
                                self.isolated_groups.insert(form.id());
                            }
                        }
                        Err(_) => self.inderterminates += 1,
                    }
                }
                Err(_) => self.inderterminates += 1,
                _ => {}
            }
        }

        // painted in isolated group and with white colorant
        let mut in_isolated_form = step.content_stack().iter().rev().filter_map(|c| {
            if let ContentReference::Form(form_id) = c
                && self.isolated_groups.contains(&form_id)
            {
                Some(true)
            } else {
                None
            }
        });

        if in_isolated_form.next().is_some() {
            match step.painted_color_spaces() {
                Ok(color_spaces) => {
                    color_spaces.colorant_names().iter().for_each(|c| {
                        if self.white_color_names.contains(&c) {
                            self.findings += 1;
                        }
                    });
                }
                Err(_) => self.inderterminates += 1,
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let pdf = Pdf::load(&args.path)?;
    let pages = pdf.pages();
    for (idx, page) in pages.into_iter().enumerate() {
        let page = page?;
        let mut isolated_white_underprints = IsolatedWhiteUnderprints::new();

        let mut walker = PageWalker::new().with_oc_config(OcConfig::default());
        walker.add_processor(&mut isolated_white_underprints);
        walker.run(&page)?;
        println!("Page {}\n======", idx);
        println!("Findings: {}", isolated_white_underprints.findings);
        println!(
            "Inderterminates: {}",
            isolated_white_underprints.inderterminates
        );
    }
    Ok(())
}
