use crate::compact_hash::Compact;
use crate::readcounts::TaxonCounters;
use crate::taxonomy::Taxonomy;
use crate::HitGroup;
use seqkmer::SpaceDist;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// fn generate_hit_string(
//     count: usize,
//     rows: &Vec<Row>,
//     taxonomy: &Taxonomy,
//     value_mask: usize,
//     offset: usize,
// ) -> String {
//     let mut result = Vec::new();
//     let mut last_pos = 0;

//     for row in rows {
//         let sort = row.kmer_id as usize;
//         if sort < offset || sort >= offset + count {
//             continue;
//         }
//         let adjusted_pos = row.kmer_id as usize - offset;

//         let value = row.value;
//         let key = value.right(value_mask);
//         let ext_code = taxonomy.nodes[key as usize].external_id;

//         if last_pos == 0 && adjusted_pos > 0 {
//             result.push((0, adjusted_pos)); // 在开始处添加0
//         } else if adjusted_pos - last_pos > 1 {
//             result.push((0, adjusted_pos - last_pos - 1)); // 在两个特定位置之间添加0
//         }
//         if let Some(last) = result.last_mut() {
//             if last.0 == ext_code {
//                 last.1 += 1;
//                 last_pos = adjusted_pos;
//                 continue;
//             }
//         }

//         // 添加当前key的计数
//         result.push((ext_code, 1));
//         last_pos = adjusted_pos;
//     }

//     // 填充尾随0
//     if last_pos < count - 1 {
//         if last_pos == 0 {
//             result.push((0, count - last_pos));
//         } else {
//             result.push((0, count - last_pos - 1));
//         }
//     }

//     result
//         .iter()
//         .map(|i| format!("{}:{}", i.0, i.1))
//         .collect::<Vec<String>>()
//         .join(" ")
// }

// &HashMap<u32, u64>,
pub fn resolve_tree(
    hit_counts: &HashMap<u32, u64>,
    taxonomy: &Taxonomy,
    required_score: u64,
) -> u32 {
    let mut max_taxon = 0u32;
    let mut max_score = 0;

    for (&taxon, _) in hit_counts {
        let mut score = 0;

        for (&taxon2, &count2) in hit_counts {
            if taxonomy.is_a_ancestor_of_b(taxon2, taxon) {
                score += count2;
            }
        }

        if score > max_score {
            max_score = score;
            max_taxon = taxon;
        } else if score == max_score {
            max_taxon = taxonomy.lca(max_taxon, taxon);
        }
    }

    max_score = *hit_counts.get(&max_taxon).unwrap_or(&0);

    while max_taxon != 0 && max_score < required_score {
        max_score = hit_counts
            .iter()
            .filter(|(&taxon, _)| taxonomy.is_a_ancestor_of_b(max_taxon, taxon))
            .map(|(_, &count)| count)
            .sum();

        if max_score >= required_score {
            break;
        }
        max_taxon = taxonomy.nodes[max_taxon as usize].parent_id as u32;
    }

    max_taxon
}

// pub fn add_hitlist_string(
//     rows: &Vec<Row>,
//     value_mask: usize,
//     kmer_count1: usize,
//     kmer_count2: Option<usize>,
//     taxonomy: &Taxonomy,
// ) -> String {
//     let result1 = generate_hit_string(kmer_count1, &rows, taxonomy, value_mask, 0);
//     if let Some(count) = kmer_count2 {
//         let result2 = generate_hit_string(count, &rows, taxonomy, value_mask, kmer_count1);
//         format!("{} |:| {}", result1, result2)
//     } else {
//         format!("{}", result1)
//     }
// }

// pub fn count_values(
//     rows: &Vec<Row>,
//     value_mask: usize,
//     kmer_count1: u32,
// ) -> (HashMap<u32, u64>, TaxonCountersDash, usize) {
//     let mut counts = HashMap::new();

//     let mut hit_count: usize = 0;

//     let mut last_row: Row = Row::new(0, 0, 0);
//     let cur_taxon_counts = TaxonCountersDash::new();

//     for row in rows {
//         let value = row.value;
//         let key = value.right(value_mask);
//         *counts.entry(key).or_insert(0) += 1;

//         // 如果切换到第2条seq,就重新计算
//         if last_row.kmer_id < kmer_count1 && row.kmer_id > kmer_count1 {
//             last_row = Row::new(0, 0, 0);
//         }
//         if !(last_row.value == value && row.kmer_id - last_row.kmer_id == 1) {
//             cur_taxon_counts
//                 .entry(key as u64)
//                 .or_default()
//                 .add_kmer(value as u64);
//             hit_count += 1;
//         }

//         last_row = *row;
//     }

//     (counts, cur_taxon_counts, hit_count)
// }

fn stat_hits<'a>(
    hits: &HitGroup,
    counts: &mut HashMap<u32, u64>,
    value_mask: usize,
    taxonomy: &Taxonomy,
    cur_taxon_counts: &mut TaxonCounters,
) -> String {
    let mut space_dist = hits.range.apply(|range| SpaceDist::new(*range));
    for row in &hits.rows {
        let value = row.value;
        let key = value.right(value_mask);

        *counts.entry(key).or_insert(0) += 1;

        cur_taxon_counts
            .entry(key as u64)
            .or_default()
            .add_kmer(value as u64);

        let ext_code = taxonomy.nodes[key as usize].external_id;
        let pos = row.kmer_id as usize;
        space_dist.add(ext_code, pos);
    }

    space_dist.fill_tail_with_zeros();
    space_dist.reduce_str(" |:| ", |str| str.to_string())
}

pub fn process_hitgroup(
    hits: &HitGroup,
    taxonomy: &Taxonomy,
    classify_counter: &AtomicUsize,
    required_score: u64,
    minimum_hit_groups: usize,
    value_mask: usize,
) -> (String, u64, String, TaxonCounters) {
    // let value_mask = hash_config.value_mask;

    let mut cur_taxon_counts = TaxonCounters::new();
    let mut counts = HashMap::new();
    let hit_groups = hits.capacity();
    let hit_string = stat_hits(
        hits,
        &mut counts,
        value_mask,
        taxonomy,
        &mut cur_taxon_counts,
    );

    // cur_counts.iter().for_each(|(key, value)| {
    //     cur_taxon_counts
    //         .entry(*key)
    //         .or_default()
    //         .merge(value)
    //         .unwrap();
    // });

    let mut call = resolve_tree(&counts, taxonomy, required_score);
    if call > 0 && hit_groups < minimum_hit_groups {
        call = 0;
    };

    let ext_call = taxonomy.nodes[call as usize].external_id;
    let clasify = if call > 0 {
        classify_counter.fetch_add(1, Ordering::SeqCst);
        cur_taxon_counts
            .entry(call as u64)
            .or_default()
            .increment_read_count();

        "C"
    } else {
        "U"
    };

    (clasify.to_owned(), ext_call, hit_string, cur_taxon_counts)
}
