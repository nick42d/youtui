use std::collections::HashMap;
use std::hash::Hash;

trait Merge {
    fn merge(&mut self, other: Self) -> Self;
}

struct Theme<T, P, C> {
    pub vals: T,
    pallette: HashMap<P, C>,
}

impl<T, P, C> Theme<T, P, C> {
    fn get_col(&self, pallete_id: &P) -> Option<&C>
    where
        P: Hash + Eq,
    {
        self.pallette.get(&pallete_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::Theme;
    use std::collections::HashMap;

    #[test]
    fn example_theme_construction() {
        #[derive(Hash, PartialEq, Eq)]
        enum ColorNames {
            BananaYellow,
            AppleRed,
        }
        struct Vals {
            on_select_colour: ColorNames,
            base_color: ColorNames,
        }
        let vals = Vals {
            on_select_colour: ColorNames::BananaYellow,
            base_color: ColorNames::AppleRed,
        };
        let pallette = HashMap::from_iter([
            (ColorNames::BananaYellow, "yellow"),
            (ColorNames::AppleRed, "red"),
        ]);
        Theme { vals, pallette };
    }
}
