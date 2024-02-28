# Shopify Price Fixer

This tool reads the output of ABC report 1-15 and compares the pricing from this report to the products on a given Shopify store. For any SKU from ABC whose price is higher than a matching Shopify product, update the shopify listing to have a corrected price.

## Installation

### Windows 

Download the latest `.exe` from [Releases](https://github.com/areif-dev/shopify-price-fixer/releases/latest)

## Building 

* Install Rust tooling from [https://rustup.rs/](https://rustup.rs/)

```bash 
git clone https://github.com/areif-dev/shopify-price-fixer
```

```bash 
cargo build
```

## Usage

### Required Configuration

The price fixer requires access to the API of your Shopify store. Provide this information in a file called `config.json` in the same folder where you stored the `shopify-price-fixer.exe` and all other script files. The format of the file is as follows:

```json
{
  "shopify_access_token": "your-super-secret-api-token",
  "business_url": "your-domain.myshopify.com",
  "storefront_url": "yourstore.com",
  "api_version": "2022-07"
}
```

For information about setting up the Shopify Admin API, see https://shopify.dev/docs/api/admin/getting-started

### Running Report 1-15 

The fixer requires ABC report 1-15 to get updated product pricing. 

In your ABC client enter: 

* F10 - to go to the main menu 
* 1 - to go to inventory reports menu
* 15 - to select INVENTORY LIST PRICES & STOCK report 
* S - to select only items that are stocked (This is optional but recommended if your store has many thousands of items)
* {Enter the starting and ending SKU}
* T - to run the report to a tab separated file 

This should generate a file called `TabOutput.tsv` in your `Documents\My ABC Files` folder. The path to this file will need to be given to the fixer. 

### Running the Fixer 

* Navigate to the location of `shopify-price-fixer.exe` in your file browser and run the application. 
* When prompted, enter or paste the path to the `TabOutput.tsv` file containing report 1-15. This will likely be something like `C:\Users\User\Documents\My ABC Files\TabOutput.tsv`

