//! Cooking: cups and spoons of an ingredient ↔ weight. The base is grams; each ingredient's
//! measures carry a tag so a cup of flour converts to grams or spoons of flour, never to sugar.

use super::Builder;
use crate::num::R;

/// (ingredient, grams per US cup of 236.6 ml, search words). Typical spooned-and-leveled values.
const INGREDIENTS: &[(&str, &str, &str)] = &[
    ("flour", "125", "all-purpose plain flour wheat baking"),
    ("whole wheat flour", "120", "wholemeal flour"),
    ("bread flour", "127", "strong flour"),
    ("sugar", "200", "granulated white sugar caster"),
    ("brown sugar", "220", "packed brown sugar"),
    ("powdered sugar", "120", "icing confectioners sugar"),
    ("butter", "227", "butter margarine"),
    ("water", "236.6", "water"),
    ("milk", "245", "milk"),
    ("heavy cream", "238", "double cream whipping"),
    ("yogurt", "245", "yoghurt"),
    ("vegetable oil", "218", "olive oil sunflower canola"),
    ("honey", "340", "honey"),
    ("maple syrup", "315", "syrup"),
    ("rice", "185", "uncooked white rice"),
    ("rolled oats", "90", "oatmeal porridge"),
    ("cocoa powder", "85", "cacao"),
    ("cornstarch", "128", "corn flour starch"),
    ("table salt", "288", "salt"),
    ("chocolate chips", "170", "chocolate"),
    ("chopped nuts", "120", "walnuts almonds pecans"),
    ("grated cheese", "100", "parmesan cheddar"),
    ("peanut butter", "258", "peanut"),
];

pub fn register(b: &mut Builder) {
    b.category("cooking", "Cooking (by ingredient)", false);
    b.linear(&[
        ("gram", "g", "1", "weight baking"),
        ("kilogram", "kg", "1000", "weight"),
        ("ounce", "oz", "28.349523125", "weight"),
        ("pound", "lb", "453.59237", "weight"),
    ]);
    for (i, &(name, per_cup, kw)) in INGREDIENTS.iter().enumerate() {
        b.tag = i as u16 + 1;
        for (measure, sym, cups) in [("cup", "cup", "1"), ("tablespoon", "tbsp", "1/16"), ("teaspoon", "tsp", "1/48"), ("milliliter", "ml", "1/236.5882365")] {
            let factor = super::expr(&format!("{per_cup}*{cups}"));
            b.unit(&format!("{measure} of {name}"), &format!("{sym} {name}"), super::Kind::Linear, factor, R::default(), &format!("{kw} cooking baking recipe {measure}s"), true);
        }
    }
    b.tag = 0;
}
