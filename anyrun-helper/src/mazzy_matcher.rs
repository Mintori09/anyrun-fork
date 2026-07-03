use std::borrow::Cow;

pub fn remove_accents(input: &str) -> Cow<'_, str> {
    // Fast path: all-ASCII input has no Vietnamese accents to remove
    if input.is_ascii() {
        return Cow::Borrowed(input);
    }

    Cow::Owned(
        input
            .chars()
            .map(|c| match c {
                'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ'
                | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
                'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => {
                    'e'
                }
                'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
                'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ'
                | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
                'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => {
                    'u'
                }
                'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
                'đ' => 'd',
                'À' | 'Á' | 'Ạ' | 'Ả' | 'Ã' | 'Â' | 'Ầ' | 'Ấ' | 'Ậ' | 'Ẩ' | 'Ẫ' | 'Ă' | 'Ằ'
                | 'Ắ' | 'Ặ' | 'Ẳ' | 'Ẵ' => 'A',
                'È' | 'É' | 'Ẹ' | 'Ẻ' | 'Ẽ' | 'Ê' | 'Ề' | 'Ế' | 'Ệ' | 'Ể' | 'Ễ' => {
                    'E'
                }
                'Ì' | 'Í' | 'Ị' | 'Ỉ' | 'Ĩ' => 'I',
                'Ò' | 'Ó' | 'Ọ' | 'Ỏ' | 'Õ' | 'Ô' | 'Ồ' | 'Ố' | 'Ộ' | 'Ổ' | 'Ỗ' | 'Ơ' | 'Ờ'
                | 'Ớ' | 'Ợ' | 'Ở' | 'Ỡ' => 'O',
                'Ù' | 'Ú' | 'Ụ' | 'Ủ' | 'Ũ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ự' | 'Ử' | 'Ữ' => {
                    'U'
                }
                'Ỳ' | 'Ý' | 'Ỵ' | 'Ỷ' | 'Ỹ' => 'Y',
                'Đ' => 'D',
                _ => c,
            })
            .collect(),
    )
}

thread_local! {
    static MATCHER: fuzzy_matcher::skim::SkimMatcherV2 =
        fuzzy_matcher::skim::SkimMatcherV2::default().smart_case();
}

pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<i64> {
    use fuzzy_matcher::FuzzyMatcher;

    // Try normal match first
    if let Some(score) = MATCHER.with(|m| m.fuzzy_match(haystack, needle)) {
        return Some(score);
    }

    // If no match, try with accents removed
    let haystack_no_accents = remove_accents(haystack);
    let needle_no_accents = remove_accents(needle);

    MATCHER.with(|m| m.fuzzy_match(&haystack_no_accents, &needle_no_accents))
}
