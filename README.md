# Shopify Price Fixer

## Mission

This software is intended to be used as a bridge between the proprietary ABC Accounting software and a Shopify website. 

## Installation

- The macro shortcut for automatically creating a properly formatted JSON file from an ABC report depends on AutoHotKey. Therefore, before running the macro, install AutoHotKey from https://www.autohotkey.com/. 
- Now install the macro by downloading or copying the contents of the export-abc-bill.ahk file in the root of the repository somewhere onto your system. To enable the script, simply double click the file to run it in the background. To Kill the script, you can right click the green "H" icon in the system tray and click "Exit". 
- Finally, install the actaul price fixer script. To install the price fixer script, go to the release section of the Github page and download the shopify-price-fixer.exe file from the latest release. At the time of publishing this README, that version is 0.2.1.

## Usage

- The price fixer uses JSON files to get the current price for an item by its sku. The file should be formatted as below
```json
[
  {"sku": "123456", "price": 12.00},
  {"sku": "789100", "price": 100.00}
]
```
While this file can be written manually by using the menus in ABC, it is recommended to use the export-abc-bill macro from this repository. 

- First, make sure that the macro is running by double clicking the export-abc-bill.ahk file wherever it is saved on the system. If the macro is already running, there is no need to follow this step
- To use the macro, first go to the F10-B or Bill entry screen on ABC.
- Navigate to the bill you wish to export
- Click on the Vendor entry field (Should be something like PURMII0 or SERCO 0)
- On the keyboard, hold down Win+Shift+E. This will begin the macro. Note that the macro will not stop until it reaches the end of the bill file, and you should not navigate away from ABC while it is running. 
- If you must interrupt the macro, you can kill it by right clicking on the AutoHotKey tray icon and clicking "Exit" or "Pause Script". The icon is a white "H" with a green background.
- The macro will generate a file on the Desktop called "exported_bill_YYYY-MM-DDTHH-MM.json" where "YYYY-MM-DDTHH-MM" will be replaced with the datestamp of when the file was created.

Now you can run the shopify-price-fixer.exe script

- Copy the path of the "exported_bill...json" file
- Run the shopify-price-fixer.exe file 
- Paste the path of the JSON file into the program when it prompts for it
- The program will query each sku to find the product information from the Shopify site, then attempt to update its price if a product and its variants can be found

## Examples

Suppose you have a report such as the following saved at C:\Users\user\Desktop\exported_bill_2022-10-21T12-23.json

### With command line args

```bash
.\price-fixer.exe C:\Users\user\Desktop\exported_bill_2022-10-21T12-23.json
```

### Without command line args

```bash
.\price-fixer.exe
Enter the path to the ABC 2-10 Report File: C:\Users\user\Desktop\exported_bill_2022-10-21T12-23.json
```
