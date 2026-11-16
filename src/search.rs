pub(crate) fn tokens(text: &str) -> Vec<String> {
    let mut split = String::new();
    let mut previous = false;
    for ch in text.chars() {
        if ch.is_uppercase() && previous {
            split.push(' ');
        }
        split.extend(ch.to_lowercase());
        previous = ch.is_lowercase();
    }
    split
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1 && !s.chars().all(|c| c.is_ascii_digit()))
        .filter(|s| {
            ![
                "the", "and", "for", "with", "from", "into", "that", "this", "how", "can", "use",
                "to", "of", "in", "on", "is", "an", "my", "it", "all",
            ]
            .contains(s)
        })
        .map(|s| {
            let s = s.strip_suffix("ing").filter(|_| s.len() > 6).unwrap_or(s);
            let s = s.strip_suffix("ed").filter(|_| s.len() > 5).unwrap_or(s);
            let s = s
                .strip_suffix('s')
                .filter(|_| s.len() > 4 && !s.ends_with("ss"))
                .unwrap_or(s);
            s.to_owned()
        })
        .collect()
}

pub(crate) fn weights(
    documents: &[[String; 6]],
) -> std::collections::BTreeMap<String, Vec<(usize, f64)>> {
    use std::collections::BTreeMap;
    let n = f64::from(u32::try_from(documents.len()).unwrap_or(u32::MAX)).max(1.0);
    let documents: Vec<_> = documents
        .iter()
        .map(|fields| fields.each_ref().map(|text| tokens(text)))
        .collect();
    let lengths = documents
        .iter()
        .map(|f| f64::from(u32::try_from(f[2].len() + f[0].len() * 3).unwrap_or(u32::MAX)))
        .collect::<Vec<_>>();
    let average = lengths.iter().sum::<f64>() / n;
    let mut table = BTreeMap::<String, Vec<(usize, f64)>>::new();
    for (i, fields) in documents.iter().enumerate() {
        let mut counts = BTreeMap::<String, f64>::new();
        for (j, field) in fields.iter().enumerate() {
            let weight = [3.0, 0.3, 1.0, 0.3, 0.5, 0.2][j];
            let unique: std::collections::BTreeSet<_> = field.iter().collect();
            for word in unique {
                *counts.entry(word.clone()).or_default() += weight;
            }
        }
        for (word, tf) in counts {
            table.entry(word).or_default().push((i, tf));
        }
    }
    for (word, postings) in &mut table {
        let df = f64::from(u32::try_from(postings.len()).unwrap_or(u32::MAX));
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
        for (i, tf) in postings {
            let name_score = if documents[*i][0].contains(word) || documents[*i][1].contains(word) {
                2.0 * idf
            } else {
                0.0
            };
            *tf = idf * *tf * 2.2 / (*tf + 1.2 * (0.25 + 0.75 * lengths[*i] / average.max(1.0)))
                + if documents[*i][0].contains(word) || documents[*i][2].contains(word) {
                    3.0
                } else {
                    idf
                }
                + name_score;
        }
    }
    table
}
