# Shopify Price Fixer

## Mission

This software is intended to be used as a bridge between the proprietary ABC Accounting software and a Shopify website.

## Installation

### Dependencies

- AutoHotKey v1.1 for controlling the screen: [https://www.autohotkey.com/](https://www.autohotkey.com/)
- Python 3.7+ for parsing ABC 2-14 report: [https://www.python.org/downloads/](https://www.python.org/downloads/)

- The script for automatically creating a properly formatted JSON file from an ABC report depends on AutoHotKey. Therefore, before running the script, install AutoHotKey v1.1 from https://www.autohotkey.com/.
- Download the price-fixer.zip file from [https://github.com/areif-dev/shopify-price-fixer/releases/latest](https://github.com/areif-dev/shopify-price-fixer/releases/latest)
- Unzip this file in a folder you will remember as you will need access to the export-abc-bill.ahk file regularly to start the script

## Usage

### Manually Export Prices

- The price fixer uses JSON files to get the current price for an item by its sku. The file should be formatted as below

```json
[
  { "sku": "123456", "price": 12.0 },
  { "sku": "789100", "price": 100.0 }
]
```

- Save this file somewhere you will remember, such as C:\Users\<username>\Desktop\exported_prices.json
- Run the `shopify-price-fixer.exe` program you installed from GitHub
- The program will prompt you for the path to a file containing exported bill info. Please provide the absolute path to this file. It should look like the following:
  ![example_shopify-price-fixer](/screenshots/shopify-price-fixer.png)

### Automatically Export Bills with AutoHotKey

- Run the export-abc-bill.ahk script by double clicking the export-abc-bill.ahk file wherever it is saved on the system
- A small menu will open that will ask for the starting and ending bill IDs. This represents the first and last bills that you want to export
  - You may omit the "Ending Bill ID" and the script will only export the Starting Bill
    ![example_export-abc-bill](/screenshots/export-bill.png)
- That is all the manual input that is required. The AHK scripts will handle generating all necessary reports, extracting necessary information from them, and submitting updated price info to Shopify. **_Do not touch the mouse or keyboard during this time!_** Doing so will interrupt the script.
