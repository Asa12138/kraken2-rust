use crate::compact_hash::Row;
use crate::utils::open_file;
// use crate::{Meros, CURRENT_REVCOM_VERSION};
use seqkmer::Meros;
use seqkmer::OptionPair;
use seqkmer::CURRENT_REVCOM_VERSION;
use std::fs::File;
use std::io::{Read, Result as IoResult, Write};
use std::mem;
use std::path::Path;

pub fn parse_binary(src: &str) -> Result<u64, std::num::ParseIntError> {
    u64::from_str_radix(src, 2)
}

pub fn construct_seed_template(minimizer_len: usize, minimizer_spaces: usize) -> String {
    if minimizer_len / 4 < minimizer_spaces {
        panic!(
            "number of minimizer spaces ({}) exceeds max for minimizer len ({}); max: {}",
            minimizer_spaces,
            minimizer_len,
            minimizer_len / 4
        );
    }
    let core = "1".repeat(minimizer_len - 2 * minimizer_spaces);
    let spaces = "01".repeat(minimizer_spaces);
    format!("{}{}", core, spaces)
}

/// 判断u64的值是否为0，并将其转换为Option<u64>类型
pub fn u64_to_option(value: u64) -> Option<u64> {
    Option::from(value).filter(|&x| x != 0)
}

pub struct HitGroup {
    pub rows: Vec<Row>,
    /// example: (0..10], 左开右闭
    pub range: OptionPair<(usize, usize)>,
}

impl HitGroup {
    pub fn new(rows: Vec<Row>, range: OptionPair<(usize, usize)>) -> Self {
        Self { rows, range }
    }

    pub fn capacity(&self) -> usize {
        self.range.reduce(0, |acc, range| acc + range.1 - range.0)
    }

    pub fn required_score(&self, confidence_threshold: f64) -> u64 {
        (confidence_threshold * self.capacity() as f64).ceil() as u64
    }
}

/// 顺序不能错
#[repr(C)]
#[derive(Debug)]
pub struct IndexOptions {
    pub k: usize,
    pub l: usize,
    pub spaced_seed_mask: u64,
    pub toggle_mask: u64,
    pub dna_db: bool,
    pub minimum_acceptable_hash_value: u64,
    pub revcom_version: i32, // 如果等于 0，就报错
    pub db_version: i32,     // 为未来的数据库结构变化预留
    pub db_type: i32,        // 为未来使用其他数据结构预留
}

impl IndexOptions {
    pub fn new(
        k: usize,
        l: usize,
        spaced_seed_mask: u64,
        toggle_mask: u64,
        dna_db: bool,
        minimum_acceptable_hash_value: u64,
    ) -> Self {
        Self {
            k,
            l,
            spaced_seed_mask,
            toggle_mask,
            dna_db,
            minimum_acceptable_hash_value,
            revcom_version: CURRENT_REVCOM_VERSION as i32,
            db_version: 0,
            db_type: 0,
        }
    }

    pub fn read_index_options<P: AsRef<Path>>(file_path: P) -> IoResult<Self> {
        let mut file = open_file(file_path)?;
        let mut buffer = vec![0; std::mem::size_of::<Self>()];
        file.read_exact(&mut buffer)?;

        let idx_opts = unsafe {
            // 确保这种转换是安全的，这依赖于数据的确切布局和来源
            std::ptr::read(buffer.as_ptr() as *const Self)
        };
        if idx_opts.revcom_version != CURRENT_REVCOM_VERSION as i32 {
            // 如果版本为0，直接触发 panic
            panic!("Unsupported version (revcom_version == 0)");
        }

        Ok(idx_opts)
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, file_path: P) -> IoResult<()> {
        let mut file = File::create(file_path)?;

        // 将结构体转换为字节切片。这是不安全的操作，因为我们正在
        // 强制将内存内容解释为字节，这要求IndexOptions是#[repr(C)]，
        // 且所有字段都可以安全地以其原始内存表示形式进行复制。
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (self as *const IndexOptions) as *const u8,
                mem::size_of::<IndexOptions>(),
            )
        };

        file.write_all(bytes)?;
        Ok(())
    }

    pub fn from_meros(meros: Meros) -> Self {
        Self::new(
            meros.k_mer,
            meros.l_mer,
            meros.spaced_seed_mask,
            meros.toggle_mask,
            true,
            meros.min_clear_hash_value.unwrap_or_default(),
        )
    }

    pub fn as_meros(&self) -> Meros {
        Meros::new(
            self.k,
            self.l,
            u64_to_option(self.spaced_seed_mask),
            u64_to_option(self.toggle_mask),
            u64_to_option(self.minimum_acceptable_hash_value),
        )
    }
}
