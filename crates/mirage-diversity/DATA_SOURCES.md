# Mirage Diversity Data Sources

The `mirage-diversity` crate bundles several compressed static assets to provide realistic population data for identity generation.

## 1. first_names.bin
- **Description:** A compressed list of ~50k given names with region weights, used to generate plausible hostnames (Pattern A and B) and Git personas.
- **Source:** Derived from Wikidata and Open Name Data.
- **License:** CC0 (Public Domain).

## 2. oui_weights.bin
- **Description:** A compressed OUI (Organizationally Unique Identifier) weight table containing the top ~200 OUIs by market share, grouped by `NicCategory` (LaptopWifi, DesktopEthernet, VmVirtio, PhoneWifi).
- **Source:** Derived from publicly available IEEE OUI data.
- **License:** Public Domain.

## 3. email_domains.bin
- **Description:** A compressed list of ~150 regional email providers (e.g., yahoo.co.jp, naver.com, 163.com) with region weights.
- **Source:** Manually curated by the Mirage project contributors.
- **License:** MIT License (as part of the Mirage project).

*Note: The GeoIP database remains the user's responsibility and is not bundled here, as per the main specification.*
