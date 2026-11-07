//! Currencies are listed offline; their rates are fetched only when converting (see `crate::rates`).

use super::{Builder, Kind};

/// code | name | search keywords (countries, signs)
const CURRENCIES: &str = "\
USD|US Dollar|$ united states america usa ecuador
EUR|Euro|€ europe germany france italy spain eurozone
UAH|Ukrainian Hryvnia|₴ ukraine гривня hryvna
GBP|British Pound|£ united kingdom england sterling
JPY|Japanese Yen|¥ japan
CNY|Chinese Yuan|¥ china renminbi rmb
CHF|Swiss Franc|switzerland
CAD|Canadian Dollar|$ canada
AUD|Australian Dollar|$ australia
NZD|New Zealand Dollar|$ new zealand
PLN|Polish Zloty|zł poland
CZK|Czech Koruna|kč czechia
HUF|Hungarian Forint|ft hungary
RON|Romanian Leu|romania
BGN|Bulgarian Lev|bulgaria
SEK|Swedish Krona|kr sweden
NOK|Norwegian Krone|kr norway
DKK|Danish Krone|kr denmark
ISK|Icelandic Krona|iceland
TRY|Turkish Lira|₺ turkey türkiye
RUB|Russian Ruble|₽ russia
BYN|Belarusian Ruble|belarus
MDL|Moldovan Leu|moldova
GEL|Georgian Lari|₾ georgia
AMD|Armenian Dram|֏ armenia
AZN|Azerbaijani Manat|₼ azerbaijan
KZT|Kazakhstani Tenge|₸ kazakhstan
KGS|Kyrgyzstani Som|kyrgyzstan
UZS|Uzbekistani Som|uzbekistan
TJS|Tajikistani Somoni|tajikistan
TMT|Turkmenistani Manat|turkmenistan
RSD|Serbian Dinar|serbia
MKD|Macedonian Denar|north macedonia
ALL|Albanian Lek|albania
BAM|Bosnia Convertible Mark|bosnia herzegovina
INR|Indian Rupee|₹ india
PKR|Pakistani Rupee|pakistan
BDT|Bangladeshi Taka|৳ bangladesh
LKR|Sri Lankan Rupee|sri lanka
NPR|Nepalese Rupee|nepal
BTN|Bhutanese Ngultrum|bhutan
MVR|Maldivian Rufiyaa|maldives
AFN|Afghan Afghani|afghanistan
IRR|Iranian Rial|iran
IQD|Iraqi Dinar|iraq
SAR|Saudi Riyal|saudi arabia
AED|UAE Dirham|united arab emirates dubai
QAR|Qatari Riyal|qatar
KWD|Kuwaiti Dinar|kuwait
BHD|Bahraini Dinar|bahrain
OMR|Omani Rial|oman
JOD|Jordanian Dinar|jordan
ILS|Israeli New Shekel|₪ israel
LBP|Lebanese Pound|lebanon
SYP|Syrian Pound|syria
YER|Yemeni Rial|yemen
EGP|Egyptian Pound|egypt
MAD|Moroccan Dirham|morocco
DZD|Algerian Dinar|algeria
TND|Tunisian Dinar|tunisia
LYD|Libyan Dinar|libya
SDG|Sudanese Pound|sudan
ETB|Ethiopian Birr|ethiopia
KES|Kenyan Shilling|kenya
UGX|Ugandan Shilling|uganda
TZS|Tanzanian Shilling|tanzania
RWF|Rwandan Franc|rwanda
BIF|Burundian Franc|burundi
SOS|Somali Shilling|somalia
DJF|Djiboutian Franc|djibouti
ERN|Eritrean Nakfa|eritrea
NGN|Nigerian Naira|₦ nigeria
GHS|Ghanaian Cedi|₵ ghana
XOF|West African CFA Franc|senegal ivory coast mali benin
XAF|Central African CFA Franc|cameroon gabon chad congo
ZAR|South African Rand|south africa
NAD|Namibian Dollar|namibia
BWP|Botswana Pula|botswana
ZMW|Zambian Kwacha|zambia
MWK|Malawian Kwacha|malawi
MZN|Mozambican Metical|mozambique
AOA|Angolan Kwanza|angola
CDF|Congolese Franc|dr congo
MGA|Malagasy Ariary|madagascar
MUR|Mauritian Rupee|mauritius
SCR|Seychellois Rupee|seychelles
SZL|Swazi Lilangeni|eswatini swaziland
LSL|Lesotho Loti|lesotho
GMD|Gambian Dalasi|gambia
GNF|Guinean Franc|guinea
SLE|Sierra Leonean Leone|sierra leone
LRD|Liberian Dollar|liberia
CVE|Cape Verdean Escudo|cape verde
STN|São Tomé Dobra|sao tome
KMF|Comorian Franc|comoros
MRU|Mauritanian Ouguiya|mauritania
SSP|South Sudanese Pound|south sudan
KRW|South Korean Won|₩ korea
KPW|North Korean Won|north korea
HKD|Hong Kong Dollar|$ hong kong
TWD|New Taiwan Dollar|taiwan
MOP|Macanese Pataca|macau
SGD|Singapore Dollar|$ singapore
MYR|Malaysian Ringgit|malaysia
IDR|Indonesian Rupiah|indonesia
THB|Thai Baht|฿ thailand
VND|Vietnamese Dong|₫ vietnam
PHP|Philippine Peso|₱ philippines
KHR|Cambodian Riel|cambodia
LAK|Lao Kip|laos
MMK|Myanmar Kyat|myanmar burma
BND|Brunei Dollar|brunei
MNT|Mongolian Tögrög|mongolia tugrik
FJD|Fijian Dollar|fiji
PGK|Papua New Guinean Kina|papua
SBD|Solomon Islands Dollar|solomon
TOP|Tongan Paʻanga|tonga
WST|Samoan Tala|samoa
VUV|Vanuatu Vatu|vanuatu
XPF|CFP Franc|french polynesia new caledonia
MXN|Mexican Peso|$ mexico
BRL|Brazilian Real|r$ brazil
ARS|Argentine Peso|$ argentina
CLP|Chilean Peso|$ chile
COP|Colombian Peso|$ colombia
PEN|Peruvian Sol|peru
UYU|Uruguayan Peso|uruguay
PYG|Paraguayan Guaraní|paraguay
BOB|Bolivian Boliviano|bolivia
VES|Venezuelan Bolívar|venezuela
GTQ|Guatemalan Quetzal|guatemala
HNL|Honduran Lempira|honduras
NIO|Nicaraguan Córdoba|nicaragua
CRC|Costa Rican Colón|costa rica
PAB|Panamanian Balboa|panama
DOP|Dominican Peso|dominican republic
CUP|Cuban Peso|cuba
JMD|Jamaican Dollar|jamaica
HTG|Haitian Gourde|haiti
TTD|Trinidad and Tobago Dollar|trinidad
BBD|Barbadian Dollar|barbados
BSD|Bahamian Dollar|bahamas
BZD|Belize Dollar|belize
XCD|East Caribbean Dollar|antigua grenada dominica
AWG|Aruban Florin|aruba
ANG|Netherlands Antillean Guilder|curacao
SRD|Surinamese Dollar|suriname
GYD|Guyanese Dollar|guyana
KYD|Cayman Islands Dollar|cayman
BMD|Bermudian Dollar|bermuda
FKP|Falkland Islands Pound|falkland
GIP|Gibraltar Pound|gibraltar
SHP|Saint Helena Pound|saint helena
";

const CRYPTO: &str = "\
BTC|Bitcoin|crypto btc xbt
ETH|Ethereum|crypto ether
USDT|Tether|crypto stablecoin
USDC|USD Coin|crypto stablecoin
BNB|BNB|crypto binance
SOL|Solana|crypto
XRP|XRP|crypto ripple
ADA|Cardano|crypto
DOGE|Dogecoin|crypto
TRX|TRON|crypto
DOT|Polkadot|crypto
LTC|Litecoin|crypto
LINK|Chainlink|crypto
AVAX|Avalanche|crypto
XLM|Stellar|crypto lumens
BCH|Bitcoin Cash|crypto
SHIB|Shiba Inu|crypto
XMR|Monero|crypto
ATOM|Cosmos|crypto
UNI|Uniswap|crypto
ETC|Ethereum Classic|crypto
DAI|Dai|crypto stablecoin
";

const METALS: &str = "\
XAU|Gold (troy ounce)|metal precious
XAG|Silver (troy ounce)|metal precious
XPT|Platinum (troy ounce)|metal precious
XPD|Palladium (troy ounce)|metal precious
";

pub fn register(b: &mut Builder) {
    b.category("currency", "Currency", true);
    for (table, common) in [(CURRENCIES, true), (CRYPTO, false), (METALS, false)] {
        for line in table.lines() {
            let mut p = line.split('|');
            let (code, name, kw) = (p.next().unwrap(), p.next().unwrap(), p.next().unwrap_or(""));
            b.add(name, code, Kind::Currency, kw, common);
        }
    }
}
