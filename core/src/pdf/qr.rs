//! codificador QR portado do `barcode/qr.go` do folio (nível M fixo, modo
//! único, menor versão possível, sem quiet zone), para gerar exatamente os
//! mesmos módulos e a mesma máscara

use thiserror::Error;

use super::content::ContentStream;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroQr {
    #[error("barcode: empty data")]
    Vazio,
    #[error("barcode: data too long for QR version 1-40 at ECC level 1")]
    Longo,
}

const NIVEL_M: usize = 1;
const ECI_UTF8: u32 = 26;
const ECI_BITS: usize = 12;
const BYTES_PAD: [u8; 2] = [0xEC, 0x11];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Modo {
    Numerico,
    Alfanumerico,
    Byte,
}

const ALFANUMERICOS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

fn valor_alfanumerico(b: u8) -> Option<u32> {
    ALFANUMERICOS
        .iter()
        .position(|&c| c == b)
        .and_then(|p| u32::try_from(p).ok())
}

fn detectar_modo(data: &[u8]) -> Modo {
    if data.iter().all(u8::is_ascii_digit) {
        Modo::Numerico
    } else if data.iter().all(|&b| valor_alfanumerico(b).is_some()) {
        Modo::Alfanumerico
    } else {
        Modo::Byte
    }
}

fn bits_contagem(modo: Modo, versao: usize) -> usize {
    let faixa = match versao {
        0..=9 => 0,
        10..=26 => 1,
        _ => 2,
    };
    match modo {
        Modo::Numerico => [10, 12, 14][faixa],
        Modo::Alfanumerico => [9, 11, 13][faixa],
        Modo::Byte => [8, 16, 16][faixa],
    }
}

fn capacidade(versao: usize, modo: Modo, eci: bool) -> usize {
    let total = DATA_CODEWORDS[versao][NIVEL_M] * 8 - 4 - bits_contagem(modo, versao);
    let total = if eci { total - ECI_BITS } else { total };
    match modo {
        Modo::Numerico => {
            let resto = total % 10;
            total / 10 * 3 + usize::from(resto >= 4) + usize::from(resto >= 7)
        }
        Modo::Alfanumerico => total / 11 * 2 + usize::from(total % 11 >= 6),
        Modo::Byte => total / 8,
    }
}

/// matriz de módulos (true = escuro), linha 0 no topo
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrCode {
    modules: Vec<Vec<bool>>,
}

type Grade = Vec<Vec<bool>>;

impl QrCode {
    pub fn new(data: &str) -> Result<Self, ErroQr> {
        let bytes = data.as_bytes();
        if bytes.is_empty() {
            return Err(ErroQr::Vazio);
        }
        let modo = detectar_modo(bytes);
        let eci = modo == Modo::Byte && !bytes.is_ascii();
        let versao = (1..=40)
            .find(|&v| bytes.len() <= capacidade(v, modo, eci))
            .ok_or(ErroQr::Longo)?;

        let size = 17 + versao * 4;
        let mut modules = vec![vec![false; size]; size];
        let mut reserved = vec![vec![false; size]; size];
        reservar_padroes(&mut modules, &mut reserved, versao, size);

        let bits = codificar(bytes, versao, modo, eci);
        posicionar_dados(&mut modules, &reserved, &bits, size);
        let (mascara, mut melhor) = avaliar_mascaras(&modules, &reserved, size);
        posicionar_formato(&mut melhor, size, mascara);
        if versao >= 7 {
            posicionar_versao(&mut melhor, size, versao);
        }
        Ok(Self { modules: melhor })
    }

    pub fn size(&self) -> usize {
        self.modules.len()
    }

    pub fn is_dark(&self, row: usize, col: usize) -> bool {
        self.modules
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .unwrap_or(false)
    }

    /// desenha como retângulos preenchidos; (x, y) é o canto inferior esquerdo
    pub fn draw(&self, stream: &mut ContentStream, x: f64, y: f64, w: f64, h: f64) {
        let n = self.size();
        if n == 0 {
            return;
        }
        let n_f = n as f64;
        let (mod_w, mod_h) = (w / n_f, h / n_f);
        stream.save_state();
        stream.set_fill_rgb([0.0, 0.0, 0.0]);
        for (row, linha) in self.modules.iter().enumerate() {
            for (col, &escuro) in linha.iter().enumerate() {
                if escuro {
                    let mx = x + col as f64 * mod_w;
                    let my = y + h - (row + 1) as f64 * mod_h;
                    stream.rectangle(mx, my, mod_w, mod_h);
                }
            }
        }
        stream.fill();
        stream.restore_state();
    }
}

fn reservar_padroes(modules: &mut Grade, reserved: &mut Grade, versao: usize, size: usize) {
    posicionar_finder(modules, reserved, 0, 0);
    posicionar_finder(modules, reserved, 0, size as isize - 7);
    posicionar_finder(modules, reserved, size as isize - 7, 0);

    if versao >= 2 {
        let posicoes = ALIGNMENT[versao];
        for &r in posicoes {
            for &c in posicoes {
                if !reserved[r][c] {
                    posicionar_alinhamento(modules, reserved, r, c);
                }
            }
        }
    }

    for i in 8..size - 8 {
        modules[6][i] = i % 2 == 0;
        reserved[6][i] = true;
        modules[i][6] = i % 2 == 0;
        reserved[i][6] = true;
    }

    modules[size - 8][8] = true;
    reserved[size - 8][8] = true;

    for i in 0..9 {
        reservar_par(reserved, 8, i);
    }
    for i in 0..8 {
        reservar_par(reserved, 8, size - 1 - i);
    }

    if versao >= 7 {
        for i in 0..6 {
            for j in 0..3 {
                reservar_par(reserved, i, size - 11 + j);
            }
        }
    }
}

/// reserva (r, c) e o simétrico (c, r)
fn reservar_par(reserved: &mut Grade, r: usize, c: usize) {
    reserved[r][c] = true;
    reserved[c][r] = true;
}

/// todas as linhas seguidas de todas as colunas, na ordem das regras do folio
fn linhas_e_colunas(m: &Grade) -> impl Iterator<Item = Vec<bool>> + '_ {
    let colunas = (0..m.len()).map(move |c| m.iter().map(|linha| linha[c]).collect());
    m.iter().cloned().chain(colunas)
}

fn posicionar_finder(modules: &mut Grade, reserved: &mut Grade, row: isize, col: isize) {
    let n = modules.len() as isize;
    for r in -1..=7isize {
        for c in -1..=7isize {
            let (rr, cc) = (row + r, col + c);
            if rr < 0 || cc < 0 || rr >= n || cc >= n {
                continue;
            }
            let dentro = (0..=6).contains(&r) && (0..=6).contains(&c);
            let escuro = dentro
                && (r == 0
                    || r == 6
                    || c == 0
                    || c == 6
                    || ((2..=4).contains(&r) && (2..=4).contains(&c)));
            let (rr, cc) = (rr as usize, cc as usize);
            modules[rr][cc] = escuro;
            reserved[rr][cc] = true;
        }
    }
}

fn posicionar_alinhamento(modules: &mut Grade, reserved: &mut Grade, row: usize, col: usize) {
    for r in -2..=2isize {
        for c in -2..=2isize {
            let (rr, cc) = (row as isize + r, col as isize + c);
            if rr < 0 || cc < 0 || rr as usize >= modules.len() || cc as usize >= modules.len() {
                continue;
            }
            let escuro = r.abs() == 2 || c.abs() == 2 || (r == 0 && c == 0);
            modules[rr as usize][cc as usize] = escuro;
            reserved[rr as usize][cc as usize] = true;
        }
    }
}

fn push_bits(bits: &mut Vec<bool>, valor: u32, n: usize) {
    for i in (0..n).rev() {
        bits.push((valor >> i) & 1 == 1);
    }
}

fn codificar_dados(data: &[u8], versao: usize, modo: Modo, eci: bool) -> Vec<bool> {
    let mut bits = Vec::new();
    let tamanho = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let digito = |b: u8| u32::from(b - b'0');
    match modo {
        Modo::Numerico => {
            push_bits(&mut bits, 0b0001, 4);
            push_bits(&mut bits, tamanho, bits_contagem(modo, versao));
            for grupo in data.chunks(3) {
                let valor = grupo.iter().fold(0, |acc, &b| acc * 10 + digito(b));
                push_bits(&mut bits, valor, [0, 4, 7, 10][grupo.len()]);
            }
        }
        Modo::Alfanumerico => {
            push_bits(&mut bits, 0b0010, 4);
            push_bits(&mut bits, tamanho, bits_contagem(modo, versao));
            for par in data.chunks(2) {
                let v = |b: u8| valor_alfanumerico(b).unwrap_or(0);
                match par {
                    [a, b] => push_bits(&mut bits, v(*a) * 45 + v(*b), 11),
                    [a] => push_bits(&mut bits, v(*a), 6),
                    _ => {}
                }
            }
        }
        Modo::Byte => {
            if eci {
                push_bits(&mut bits, 0b0111, 4);
                push_bits(&mut bits, ECI_UTF8, 8);
            }
            push_bits(&mut bits, 0b0100, 4);
            push_bits(&mut bits, tamanho, bits_contagem(modo, versao));
            for &b in data {
                push_bits(&mut bits, u32::from(b), 8);
            }
        }
    }
    bits
}

fn codificar(data: &[u8], versao: usize, modo: Modo, eci: bool) -> Vec<bool> {
    let mut bits = codificar_dados(data, versao, modo, eci);
    let total = DATA_CODEWORDS[versao][NIVEL_M] * 8;
    for _ in 0..4 {
        if bits.len() >= total {
            break;
        }
        bits.push(false);
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(false);
    }
    let mut pad = 0;
    while bits.len() < total {
        push_bits(&mut bits, u32::from(BYTES_PAD[pad % 2]), 8);
        pad += 1;
    }
    bits.truncate(total);
    intercalar_ecc(&bits, versao)
}

fn intercalar_ecc(bits: &[bool], versao: usize) -> Vec<bool> {
    let dados: Vec<u8> = bits
        .chunks(8)
        .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b)))
        .collect();
    let (g1, cw1, g2, cw2) = BLOCKS[versao][NIVEL_M];
    let ecc_por_bloco = ECC_PER_BLOCK[versao][NIVEL_M];

    let mut blocos: Vec<&[u8]> = Vec::with_capacity(g1 + g2);
    let mut offset = 0;
    for n in std::iter::repeat_n(cw1, g1).chain(std::iter::repeat_n(cw2, g2)) {
        blocos.push(&dados[offset..offset + n]);
        offset += n;
    }
    let gf = GaloisField::new();
    let gerador = gf.gerador(ecc_por_bloco);
    let eccs: Vec<Vec<u8>> = blocos
        .iter()
        .map(|b| gf.rs_encode(b, &gerador, ecc_por_bloco))
        .collect();

    let max_cw = if g2 > 0 { cw1.max(cw2) } else { cw1 };
    let mut intercalado: Vec<u8> = Vec::new();
    for j in 0..max_cw {
        intercalado.extend(blocos.iter().filter_map(|b| b.get(j)));
    }
    for j in 0..ecc_por_bloco {
        intercalado.extend(eccs.iter().filter_map(|e| e.get(j)));
    }
    let mut out = Vec::with_capacity(intercalado.len() * 8);
    for b in intercalado {
        push_bits(&mut out, u32::from(b), 8);
    }
    out
}

struct GaloisField {
    exp: [u8; 512],
    log: [u8; 256],
}

impl GaloisField {
    fn new() -> Self {
        let (mut exp, mut log) = ([0u8; 512], [0u8; 256]);
        let mut x: u16 = 1;
        for i in 0..255u8 {
            exp[usize::from(i)] = x as u8;
            log[usize::from(x)] = i;
            x <<= 1;
            if x >= 256 {
                x ^= 0x11D;
            }
        }
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        Self { exp, log }
    }

    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        self.exp[usize::from(self.log[usize::from(a)]) + usize::from(self.log[usize::from(b)])]
    }

    fn gerador(&self, n: usize) -> Vec<u8> {
        let mut g = vec![1u8];
        for i in 0..n {
            let mut ng = vec![0u8; g.len() + 1];
            for (j, &coef) in g.iter().enumerate() {
                ng[j] ^= coef;
                ng[j + 1] ^= self.mul(coef, self.exp[i]);
            }
            g = ng;
        }
        g
    }

    fn rs_encode(&self, data: &[u8], gerador: &[u8], ecc: usize) -> Vec<u8> {
        let mut msg = data.to_vec();
        msg.resize(data.len() + ecc, 0);
        for i in 0..data.len() {
            let coef = msg[i];
            if coef == 0 {
                continue;
            }
            for (j, &g) in gerador.iter().enumerate() {
                msg[i + j] ^= self.mul(g, coef);
            }
        }
        msg.split_off(data.len())
    }
}

fn posicionar_dados(modules: &mut Grade, reserved: &Grade, bits: &[bool], size: usize) {
    let mut idx = 0;
    let mut subindo = true;
    let mut col = size as isize - 1;
    while col >= 0 {
        if col == 6 {
            col -= 1;
        }
        let c0 = col as usize;
        for i in 0..size {
            let row = if subindo { size - 1 - i } else { i };
            for c in (c0.saturating_sub(1)..=c0).rev() {
                if reserved[row][c] {
                    continue;
                }
                if let Some(&b) = bits.get(idx) {
                    modules[row][c] = b;
                    idx += 1;
                }
            }
        }
        subindo = !subindo;
        col -= 2;
    }
}

fn mascara(m: usize, row: usize, col: usize) -> bool {
    match m {
        0 => (row + col).is_multiple_of(2),
        1 => row.is_multiple_of(2),
        2 => col.is_multiple_of(3),
        3 => (row + col).is_multiple_of(3),
        4 => (row / 2 + col / 3).is_multiple_of(2),
        5 => (row * col) % 2 + (row * col) % 3 == 0,
        6 => ((row * col) % 2 + (row * col) % 3).is_multiple_of(2),
        7 => ((row * col) % 3 + (row + col) % 2).is_multiple_of(2),
        _ => false,
    }
}

fn aplicar_mascara(modules: &Grade, reserved: &Grade, m: usize) -> Grade {
    modules
        .iter()
        .enumerate()
        .map(|(r, linha)| {
            linha
                .iter()
                .enumerate()
                .map(|(c, &v)| v ^ (!reserved[r][c] && mascara(m, r, c)))
                .collect()
        })
        .collect()
}

fn avaliar_mascaras(modules: &Grade, reserved: &Grade, size: usize) -> (usize, Grade) {
    let mut melhor = (0, usize::MAX, modules.clone());
    for m in 0..8 {
        let mascarado = aplicar_mascara(modules, reserved, m);
        let p = penalidade(&mascarado, size);
        if p < melhor.1 {
            melhor = (m, p, mascarado);
        }
    }
    (melhor.0, melhor.2)
}

fn penalidade(m: &Grade, size: usize) -> usize {
    regra1(m) + regra2(m, size) + regra3(m) + regra4(m, size)
}

fn corrida(len: usize) -> usize {
    if len >= 5 { 3 + (len - 5) } else { 0 }
}

fn regra1(m: &Grade) -> usize {
    let mut p = 0;
    for seq in linhas_e_colunas(m) {
        let mut count = 1;
        for par in seq.windows(2) {
            if par[0] == par[1] {
                count += 1;
            } else {
                p += corrida(count);
                count = 1;
            }
        }
        p += corrida(count);
    }
    p
}

fn regra2(m: &Grade, size: usize) -> usize {
    let mut count = 0;
    for r in 0..size - 1 {
        for c in 0..size - 1 {
            let v = m[r][c];
            if m[r][c + 1] == v && m[r + 1][c] == v && m[r + 1][c + 1] == v {
                count += 1;
            }
        }
    }
    3 * count
}

const PADRAO1: [bool; 11] = [
    true, false, true, true, true, false, true, false, false, false, false,
];
const PADRAO2: [bool; 11] = [
    false, false, false, false, true, false, true, true, true, false, true,
];

fn regra3(m: &Grade) -> usize {
    let count: usize = linhas_e_colunas(m)
        .map(|seq| {
            seq.windows(PADRAO1.len())
                .filter(|j| *j == PADRAO1 || *j == PADRAO2)
                .count()
        })
        .sum();
    40 * count
}

fn regra4(m: &Grade, size: usize) -> usize {
    let escuros = m.iter().flatten().filter(|&&v| v).count();
    let percentual = escuros * 100 / (size * size);
    let diff = percentual.abs_diff(50);
    10 * ((diff / 5) * 2)
}

fn posicionar_formato(modules: &mut Grade, size: usize, m: usize) {
    let info = FORMAT_INFO[NIVEL_M][m];
    let bits: Vec<bool> = (0..15).map(|i| (info >> (14 - i)) & 1 == 1).collect();
    const POSICOES: [(usize, usize); 15] = [
        (0, 8),
        (1, 8),
        (2, 8),
        (3, 8),
        (4, 8),
        (5, 8),
        (7, 8),
        (8, 8),
        (8, 7),
        (8, 5),
        (8, 4),
        (8, 3),
        (8, 2),
        (8, 1),
        (8, 0),
    ];
    for (i, (r, c)) in POSICOES.iter().enumerate() {
        modules[*r][*c] = bits[14 - i];
    }
    for i in 0..7 {
        modules[size - 1 - i][8] = bits[i];
    }
    for i in 0..8 {
        modules[8][size - 8 + i] = bits[7 + i];
    }
}

fn posicionar_versao(modules: &mut Grade, size: usize, versao: usize) {
    let info = VERSION_INFO[versao];
    for i in 0..18 {
        let bit = (info >> i) & 1 == 1;
        let (r, c) = (i / 3, i % 3);
        modules[size - 11 + c][r] = bit;
        modules[r][size - 11 + c] = bit;
    }
}

const DATA_CODEWORDS: [[usize; 4]; 41] = [
    [0, 0, 0, 0],
    [19, 16, 13, 9],
    [34, 28, 22, 16],
    [55, 44, 34, 26],
    [80, 64, 48, 36],
    [108, 86, 62, 46],
    [136, 108, 76, 60],
    [156, 124, 88, 66],
    [194, 154, 110, 86],
    [232, 182, 132, 100],
    [274, 216, 154, 122],
    [324, 254, 180, 140],
    [370, 290, 206, 158],
    [428, 334, 244, 180],
    [461, 365, 261, 197],
    [523, 415, 295, 223],
    [589, 453, 325, 253],
    [647, 507, 367, 283],
    [721, 563, 397, 313],
    [795, 627, 445, 341],
    [861, 669, 485, 385],
    [932, 714, 512, 406],
    [1006, 782, 568, 442],
    [1094, 860, 614, 464],
    [1174, 914, 664, 514],
    [1276, 1000, 718, 538],
    [1370, 1062, 754, 596],
    [1468, 1128, 808, 628],
    [1531, 1193, 871, 661],
    [1631, 1267, 911, 701],
    [1735, 1373, 985, 745],
    [1843, 1455, 1033, 793],
    [1955, 1541, 1115, 845],
    [2071, 1631, 1171, 901],
    [2191, 1725, 1231, 961],
    [2306, 1812, 1286, 986],
    [2434, 1914, 1354, 1054],
    [2566, 1992, 1426, 1096],
    [2702, 2102, 1502, 1142],
    [2812, 2216, 1582, 1222],
    [2956, 2334, 1666, 1276],
];

const ECC_PER_BLOCK: [[usize; 4]; 41] = [
    [0, 0, 0, 0],
    [7, 10, 13, 17],
    [10, 16, 22, 28],
    [15, 26, 18, 22],
    [20, 18, 26, 16],
    [26, 24, 18, 22],
    [18, 16, 24, 28],
    [20, 18, 18, 26],
    [24, 22, 22, 26],
    [30, 22, 20, 24],
    [18, 26, 24, 28],
    [20, 30, 28, 24],
    [24, 22, 26, 28],
    [26, 22, 24, 22],
    [30, 24, 20, 24],
    [22, 24, 30, 24],
    [24, 28, 24, 30],
    [28, 28, 28, 28],
    [30, 26, 28, 28],
    [28, 26, 26, 26],
    [28, 26, 30, 28],
    [28, 26, 28, 30],
    [28, 28, 30, 24],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [26, 28, 30, 30],
    [28, 28, 28, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
    [30, 28, 30, 30],
];

type Bloco = (usize, usize, usize, usize);

const BLOCKS: [[Bloco; 4]; 41] = [
    [(0, 0, 0, 0); 4],
    [(1, 19, 0, 0), (1, 16, 0, 0), (1, 13, 0, 0), (1, 9, 0, 0)],
    [(1, 34, 0, 0), (1, 28, 0, 0), (1, 22, 0, 0), (1, 16, 0, 0)],
    [(1, 55, 0, 0), (1, 44, 0, 0), (2, 17, 0, 0), (2, 13, 0, 0)],
    [(1, 80, 0, 0), (2, 32, 0, 0), (2, 24, 0, 0), (4, 9, 0, 0)],
    [
        (1, 108, 0, 0),
        (2, 43, 0, 0),
        (2, 15, 2, 16),
        (2, 11, 2, 12),
    ],
    [(2, 68, 0, 0), (4, 27, 0, 0), (4, 19, 0, 0), (4, 15, 0, 0)],
    [(2, 78, 0, 0), (4, 31, 0, 0), (2, 14, 4, 15), (4, 13, 1, 14)],
    [
        (2, 97, 0, 0),
        (2, 38, 2, 39),
        (4, 18, 2, 19),
        (4, 14, 2, 15),
    ],
    [
        (2, 116, 0, 0),
        (3, 36, 2, 37),
        (4, 16, 4, 17),
        (4, 12, 4, 13),
    ],
    [
        (2, 68, 2, 69),
        (4, 43, 1, 44),
        (6, 19, 2, 20),
        (6, 15, 2, 16),
    ],
    [
        (4, 81, 0, 0),
        (1, 50, 4, 51),
        (4, 22, 4, 23),
        (3, 12, 8, 13),
    ],
    [
        (2, 92, 2, 93),
        (6, 36, 2, 37),
        (4, 20, 6, 21),
        (7, 14, 4, 15),
    ],
    [
        (4, 107, 0, 0),
        (8, 37, 1, 38),
        (8, 20, 4, 21),
        (12, 11, 4, 12),
    ],
    [
        (3, 115, 1, 116),
        (4, 40, 5, 41),
        (11, 16, 5, 17),
        (11, 12, 5, 13),
    ],
    [
        (5, 87, 1, 88),
        (5, 41, 5, 42),
        (5, 24, 7, 25),
        (11, 12, 7, 13),
    ],
    [
        (5, 98, 1, 99),
        (7, 45, 3, 46),
        (15, 19, 2, 20),
        (3, 15, 13, 16),
    ],
    [
        (1, 107, 5, 108),
        (10, 46, 1, 47),
        (1, 22, 15, 23),
        (2, 14, 17, 15),
    ],
    [
        (5, 120, 1, 121),
        (9, 43, 4, 44),
        (17, 22, 1, 23),
        (2, 14, 19, 15),
    ],
    [
        (3, 113, 4, 114),
        (3, 44, 11, 45),
        (17, 21, 4, 22),
        (9, 13, 16, 14),
    ],
    [
        (3, 107, 5, 108),
        (3, 41, 13, 42),
        (15, 24, 5, 25),
        (15, 15, 10, 16),
    ],
    [
        (4, 116, 4, 117),
        (17, 42, 0, 0),
        (17, 22, 6, 23),
        (19, 16, 6, 17),
    ],
    [
        (2, 111, 7, 112),
        (17, 46, 0, 0),
        (7, 24, 16, 25),
        (34, 13, 0, 0),
    ],
    [
        (4, 121, 5, 122),
        (4, 47, 14, 48),
        (11, 24, 14, 25),
        (16, 15, 14, 16),
    ],
    [
        (6, 117, 4, 118),
        (6, 45, 14, 46),
        (11, 24, 16, 25),
        (30, 16, 2, 17),
    ],
    [
        (8, 106, 4, 107),
        (8, 47, 13, 48),
        (7, 24, 22, 25),
        (22, 15, 13, 16),
    ],
    [
        (10, 114, 2, 115),
        (19, 46, 4, 47),
        (28, 22, 6, 23),
        (33, 16, 4, 17),
    ],
    [
        (8, 122, 4, 123),
        (22, 45, 3, 46),
        (8, 23, 26, 24),
        (12, 15, 28, 16),
    ],
    [
        (3, 117, 10, 118),
        (3, 45, 23, 46),
        (4, 24, 31, 25),
        (11, 15, 31, 16),
    ],
    [
        (7, 116, 7, 117),
        (21, 45, 7, 46),
        (1, 23, 37, 24),
        (19, 15, 26, 16),
    ],
    [
        (5, 115, 10, 116),
        (19, 47, 10, 48),
        (15, 24, 25, 25),
        (23, 15, 25, 16),
    ],
    [
        (13, 115, 3, 116),
        (2, 46, 29, 47),
        (42, 24, 1, 25),
        (23, 15, 28, 16),
    ],
    [
        (17, 115, 0, 0),
        (10, 46, 23, 47),
        (10, 24, 35, 25),
        (19, 15, 35, 16),
    ],
    [
        (17, 115, 1, 116),
        (14, 46, 21, 47),
        (29, 24, 19, 25),
        (11, 15, 46, 16),
    ],
    [
        (13, 115, 6, 116),
        (14, 46, 23, 47),
        (44, 24, 7, 25),
        (59, 16, 1, 17),
    ],
    [
        (12, 121, 7, 122),
        (12, 47, 26, 48),
        (39, 24, 14, 25),
        (22, 15, 41, 16),
    ],
    [
        (6, 121, 14, 122),
        (6, 47, 34, 48),
        (46, 24, 10, 25),
        (2, 15, 64, 16),
    ],
    [
        (17, 122, 4, 123),
        (29, 46, 14, 47),
        (49, 24, 10, 25),
        (24, 15, 46, 16),
    ],
    [
        (4, 122, 18, 123),
        (13, 46, 32, 47),
        (48, 24, 14, 25),
        (42, 15, 32, 16),
    ],
    [
        (20, 117, 4, 118),
        (40, 47, 7, 48),
        (43, 24, 22, 25),
        (10, 15, 67, 16),
    ],
    [
        (19, 118, 6, 119),
        (18, 47, 31, 48),
        (34, 24, 34, 25),
        (20, 15, 61, 16),
    ],
];

const ALIGNMENT: [&[usize]; 41] = [
    &[],
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
    &[6, 30, 54],
    &[6, 32, 58],
    &[6, 34, 62],
    &[6, 26, 46, 66],
    &[6, 26, 48, 70],
    &[6, 26, 50, 74],
    &[6, 30, 54, 78],
    &[6, 30, 56, 82],
    &[6, 30, 58, 86],
    &[6, 34, 62, 90],
    &[6, 28, 50, 72, 94],
    &[6, 26, 50, 74, 98],
    &[6, 30, 54, 78, 102],
    &[6, 28, 54, 80, 106],
    &[6, 32, 58, 84, 110],
    &[6, 30, 58, 86, 114],
    &[6, 34, 62, 90, 118],
    &[6, 26, 50, 74, 98, 122],
    &[6, 30, 54, 78, 102, 126],
    &[6, 26, 52, 78, 104, 130],
    &[6, 30, 56, 82, 108, 134],
    &[6, 34, 60, 86, 112, 138],
    &[6, 30, 58, 86, 114, 142],
    &[6, 34, 62, 90, 118, 146],
    &[6, 30, 54, 78, 102, 126, 150],
    &[6, 24, 50, 76, 102, 128, 154],
    &[6, 28, 54, 80, 106, 132, 158],
    &[6, 32, 58, 84, 110, 136, 162],
    &[6, 26, 54, 82, 110, 138, 166],
    &[6, 30, 58, 86, 114, 142, 170],
];

const FORMAT_INFO: [[u16; 8]; 4] = [
    [
        0x77C4, 0x72F3, 0x7DAA, 0x789D, 0x662F, 0x6318, 0x6C41, 0x6976,
    ],
    [
        0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0,
    ],
    [
        0x355F, 0x3068, 0x3F31, 0x3A06, 0x24B4, 0x2183, 0x2EDA, 0x2BED,
    ],
    [
        0x1689, 0x13BE, 0x1CE7, 0x19D0, 0x0762, 0x0255, 0x0D0C, 0x083B,
    ],
];

const VERSION_INFO: [u32; 41] = [
    0, 0, 0, 0, 0, 0, 0, 0x07C94, 0x085BC, 0x09A99, 0x0A4D3, 0x0BBF6, 0x0C762, 0x0D847, 0x0E60D,
    0x0F928, 0x10B78, 0x1145D, 0x12A17, 0x13532, 0x149A6, 0x15683, 0x168C9, 0x177EC, 0x18EC4,
    0x191E1, 0x1AFAB, 0x1B08E, 0x1CC1A, 0x1D33F, 0x1ED75, 0x1F250, 0x209D5, 0x216F0, 0x228BA,
    0x2379F, 0x24B0B, 0x2542E, 0x26A64, 0x27541, 0x28C69,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejeita_vazio() {
        assert_eq!(QrCode::new(""), Err(ErroQr::Vazio));
    }

    #[test]
    fn escolhe_menor_versao() {
        let curto = QrCode::new("123").map(|q| q.size());
        assert_eq!(curto, Ok(21));
        let url = "https://www.nfse.gov.br/ConsultaPublica/?tpc=1&chave=35503082212345678000195000000000000126070000000001";
        assert_eq!(QrCode::new(url).map(|q| q.size()), Ok(41));
        assert_eq!(QrCode::new(&"a".repeat(3000)), Err(ErroQr::Longo));
    }

    #[test]
    fn padroes_finder_presentes() {
        let Ok(q) = QrCode::new("HELLO WORLD") else {
            panic!("qr inválido");
        };
        let n = q.size();
        for (r, c) in [(0, 0), (0, n - 1), (n - 1, 0), (3, 3), (n - 4, 3)] {
            assert!(q.is_dark(r, c), "({r},{c})");
        }
        assert!(!q.is_dark(7, 7));
        assert!(q.is_dark(n - 8, 8));
    }

    #[test]
    fn versao_grande_tem_info_de_versao() {
        assert!(
            QrCode::new(&"a".repeat(200))
                .map(|q| q.size())
                .is_ok_and(|s| s >= 45)
        );
    }
}
