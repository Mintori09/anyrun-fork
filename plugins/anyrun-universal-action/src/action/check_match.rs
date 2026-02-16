use crate::action::model::{InputCategory, UniversalAction};

impl UniversalAction {
    pub fn is_match(&self, clipboard: &str, detected_cat: InputCategory) -> bool {
        let category_ok = match (&self.category, &detected_cat) {
            (
                InputCategory::Code { lang: cfg_lang, .. },
                InputCategory::Code { lang: det_lang, .. },
            ) => {
                cfg_lang == "any"
                    || cfg_lang.is_empty()
                    || cfg_lang == "all"
                    || cfg_lang == det_lang
            }

            (c1, c2) => c1 == c2,
        };

        if !category_ok {
            return false;
        }

        if let Some(validator_fn) = self.validator {
            return validator_fn(clipboard);
        }

        true
    }
}
