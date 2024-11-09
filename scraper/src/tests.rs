use crate::{currency, identify::*, prelude::*, tokenize::*};

const TEST_DATA: &[[&str; 3]] = &[
    [
        "FS: Panerai Luminor Marina TuttoGrigio Titanium & Carbotech PAM01662",
        "pam01662",
        "Panerai",
    ],
    [
        "FS: Patek Philippe Complications Chronograph 5172G-001",
        "5172g-001",
        "Patek Philippe",
    ],
    [
        "FS: IWC Big Pilots TOP GUN Ceratanium Double Chrono IW371815",
        "iw371815",
        "IWC",
    ],
    ["Omega speedmaster nib", "", "Omega"],
    [
        "FS: IWC Portuguese Yacht Club Chronograph Certified Steel Black 45mm IW390204 Rubber",
        "iw390204",
        "IWC",
    ],
    [
        "FSOT: A. Lange & Sohne LANGE 1 191.032 ROSE GOLD 38.5MM 2023 WARRANTY FULL SET",
        "191.032",
        "A. Lange & Söhne",
    ],
    [
        "Cartier Tortue Certified Large 18k Rose Gold Factory Diamonds 43mm WA503951 2498",
        "wa503951",
        "Cartier",
    ],
    [
        "FS: Cartier Tank Solo Certified XL 18k Rose Gold Steel W5200026 3799 Automatic",
        "w5200026",
        "Cartier",
    ],
    [
        "FS: Heuer Autavia Valjoux Circa 1972 73663",
        "73663",
        "Heuer",
    ],
    [
        "FS: 2023 126508 Rolex Daytona Yellow Gold \"Pikachu\" EXCELLENT CONDITON/COMPLETE SET",
        "126508",
        "Rolex",
    ],
    [
        "Tudor Black Bay Heritage 41mm | 79230R - Full Set",
        "79230r",
        "TUDOR",
    ],
    [
        "FS: ROLEX 79160 Ladies Steel Date Silver Index",
        "79160",
        "Rolex",
    ],
    [
        "FSOT: Rolex 126300 DATEJUST 41 BLUE STICK DIAL JUBILEE BAND 2024 COMPLETE SET",
        "126300",
        "Rolex",
    ],
];

#[test]
fn model_no() {
    let empty = "".to_owned().into_boxed_str();

    for &[s, b, _] in TEST_DATA {
        let t = tokenize_watch_info(s);
        let a = find_model_no(&t).unwrap_or(&empty);

        // println!("{a:?}");
        assert_eq!(&**a, b);
    }
}

#[test]
fn brand_name() {
    let empty = "".to_owned().into_boxed_str();

    for &[s, _, b] in TEST_DATA {
        let t = tokenize_watch_info(s);
        let a = find_brand(&t).unwrap_or(&empty);

        // println!("{a:?}");
        assert_eq!(a, b);
    }
}

#[test]
fn extract_currency() {
    // // test extra strings before and after currency value
    // assert_eq!(
    //     currency::extract_currency("before 42.32 USD").ok(),
    //     Some(("USD", "42.32".to_owned()))
    // );
    // assert_eq!(
    //     currency::extract_currency("42.32 USD after").ok(),
    //     Some(("USD", "42.32".to_owned()))
    // );
    // // test extracting currency code
    // assert_eq!(
    //     currency::extract_currency("42.32 usd").ok(),
    //     Some(("USD", "42.32".into()))
    // );
    // assert_eq!(
    //     currency::extract_currency("42.32 USD").ok(),
    //     Some(("USD", "42.32".into()))
    // );
}

#[test]
fn currency() {
    // use currency::Currency;
    // use num::traits::ToPrimitive;
    //
    // println!("{}", extract_currency("lmaoo 42.32 USD"));
    // println!("{}", extract_currency("lmaoo USD 42.32"));
    //
    // let currencies = [
    //     Currency::from_str("$42.32").unwrap(),
    //     Currency::from_str("$ 42.32").unwrap(),
    //     Currency::from_str(&normalize_currency("42.32 USD")).unwrap(),
    //     Currency::from_str(&normalize_currency("USD 42.32")).unwrap(),
    // ];
    //
    // for currency in currencies {
    //     println!("{currency:?}");
    // }

    //assert_eq!(c_usd1, 4232);
}
