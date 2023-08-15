use crate::compiler::Token;

macro_rules! build_tokens {
    ($arr_var:ident, $($word:literal),+) => {
        const $arr_var : [&str; $({$word; 1}+)+0] = [
            $($word),+
        ];
    };
}

build_tokens!(OPERATIONS, "create", "add_row", "headers", "apply", "view");
build_tokens!(MODIFIERS, "with", "and");
build_tokens!(DMODIFIERS, "title");

// Note: Current implementation assumes the keyboard layout uses keys such that they have a
// contiguous range of UTF encoding, which is true for english language.
struct KeyBoard<const N: usize, const P: usize> {
    distance_map: [[usize; N]; N],
    first_char: i64,
}

impl<const N: usize, const P: usize> KeyBoard<N, P> {
    pub const fn lookup(&self, a: char, b: char) -> Option<u32> {
        // only ascii lookup to be made
        if !a.is_ascii_alphabetic() || !b.is_ascii_alphabetic() {
            return None;
        }
        let a = a.to_ascii_lowercase() as i64 - self.first_char;
        let b = b.to_ascii_lowercase() as i64 - self.first_char;
        Some(self.distance_map[a as usize][b as usize] as u32)
    }
    pub const fn new(layout: &[[char; P]; N]) -> Self {
        let mut smallest = 'z' as usize;
        let mut largest = 0 as usize;
        let mut i = 0;
        while i < N {
            let mut j = 0;
            while j < P {
                if (layout[i][j] as usize) < smallest {
                    smallest = layout[i][j] as usize;
                }
                if (layout[i][j] as usize) > largest {
                    largest = layout[i][j] as usize;
                }
                j += 1;
            }
            i += 1;
        }
        let smallest_char = smallest as usize;
        let largest_char = largest as usize;
        assert!(largest_char + 1 == N + smallest_char);
        let mut new_layout = [[0; P]; N];
        let mut i = 0;
        while i < N {
            let mut j = 0;
            while j < P {
                new_layout[i][j] = layout[i][j] as usize - smallest_char;
                j += 1;
            }
            i += 1;
        }
        let adjacency_list = new_layout;
        let mut start = 0;
        let mut distances = [[usize::MAX; N]; N];

        while start < N {
            let mut queue = [0; 1000];
            let mut front = 0;
            let mut rear = 0;
            queue[rear] = start;
            rear += 1;

            distances[start][start] = 0;
            let mut curr_dist = 0;
            while rear - front > 0 {
                let mut k = rear - front;
                while k > 0 {
                    k -= 1;
                    curr_dist += 1;
                    let current_node = queue[front];
                    front += 1;

                    let neighbors = &adjacency_list[current_node];
                    let mut i = 0;

                    while i < P {
                        let neighbor = neighbors[i];
                        i += 1;

                        if distances[start][neighbor] == usize::MAX {
                            distances[start][neighbor] = curr_dist;
                            queue[rear] = neighbor;
                            rear += 1;
                        } else if distances[start][neighbor] > distances[start][current_node] + 1 {
                            distances[start][neighbor] = distances[start][current_node] + 1;
                        }
                    }
                }
            }
            start += 1;
        }
        KeyBoard {
            distance_map: distances,
            first_char: smallest as i64,
        }
    }
}

const fn build_keyboard_distances() -> KeyBoard<26, 6> {
    let keyboard = [
        ['q', 'w', 's', 'z', 'x', 'x'], // a
        ['g', 'h', 'v', 'n', 'n', 'n'], // b
        ['x', 'd', 'f', 'v', 'v', 'v'], // c
        ['s', 'e', 'r', 'f', 'c', 'x'], // d
        ['r', 'w', 's', 'd', 'f', 'f'], // e
        ['r', 'd', 'g', 't', 'c', 'v'], // f
        ['t', 'y', 'h', 'b', 'v', 'f'], // g
        ['g', 'y', 'u', 'j', 'n', 'b'], // h
        ['u', 'j', 'k', 'l', 'o', 'o'], // i
        ['h', 'u', 'i', 'k', 'm', 'n'], // j
        ['j', 'i', 'o', 'l', 'm', 'm'], // k
        ['p', 'o', 'k', 'm', 'm', 'm'], // l
        ['n', 'j', 'k', 'l', 'l', 'l'], // m
        ['b', 'h', 'j', 'k', 'm', 'm'], // n
        ['i', 'p', 'k', 'l', 'l', 'l'], // o
        ['o', 'l', 'l', 'l', 'l', 'l'], // p
        ['a', 'a', 'a', 'a', 's', 'w'], // q
        ['e', 'd', 'f', 'g', 't', 't'], // r
        ['a', 'w', 'e', 'd', 'x', 'z'], // s
        ['f', 'g', 'h', 'y', 'r', 'r'], // t
        ['y', 'h', 'j', 'k', 'i', 'i'], // u
        ['c', 'f', 'g', 'b', 'b', 'b'], // v
        ['q', 'a', 's', 'd', 'e', 'e'], // w
        ['z', 's', 'd', 'c', 'c', 'c'], // x
        ['t', 'g', 'h', 'j', 'u', 'u'], // y
        ['a', 's', 'x', 'x', 'x', 'x'], // z
    ];
    KeyBoard::new(&keyboard)
}

const KEYBOARD: KeyBoard<26, 6> = build_keyboard_distances();
pub(crate) fn degree_of_closeness(key: &str, matcher: &str) -> u32 {
    /*
     * this returns a fictional cost of changing key to matcher
     * the smaller the better
     *
     * one can use dynamic programming to make this faster
     * but we are actually working an really small strings,
     * so it is not worth it
     */
    if key.is_empty() || matcher.is_empty() {
        return ((key.len() + matcher.len()) << 3) as u32;
    };
    let (key_c, match_c) = (key.chars().next().unwrap(), matcher.chars().next().unwrap());
    if key_c == match_c {
        return degree_of_closeness(&key[1..], &matcher[1..]);
    }
    if !key_c.is_ascii_alphabetic() {
        return degree_of_closeness(&key[1..], matcher);
    }
    if !match_c.is_ascii_alphabetic() {
        return degree_of_closeness(key, &matcher[1..]);
    }
    let delete_cost = degree_of_closeness(&key[1..], matcher) + (1 << 3);
    let insert_cost = degree_of_closeness(key, &matcher[1..]) + (1 << 3);
    let replace_cost = degree_of_closeness(&key[1..], &matcher[1..])
        + KEYBOARD.lookup(key_c, match_c).unwrap_or(0);
    delete_cost.min(insert_cost).min(replace_cost)
}

pub(crate) fn keyboard_distance_matcher<'a>(token: &'a str, token_type: Token<'a>) -> &'a str {
    match token_type {
        Token::Operator => OPERATIONS
            .iter()
            .min_by_key(|&op| degree_of_closeness(op, token))
            .unwrap(),
        Token::Modifier => MODIFIERS
            .iter()
            .min_by_key(|&op| degree_of_closeness(op, token))
            .unwrap(),
        Token::DataModifier => DMODIFIERS
            .iter()
            .min_by_key(|&op| degree_of_closeness(op, token))
            .unwrap(),
        Token::Table(table_idx) => table_idx
            .keys()
            .min_by_key(|&op| degree_of_closeness(op, token))
            .unwrap(),
    }
}
