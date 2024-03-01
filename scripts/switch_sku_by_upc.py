import csv
import requests 
import json 

def update_shopify_sku(variant: int, new_sku: str):
    config = get_config()
    access_token = config["shopify_access_token"]
    business_url = config["business_url"]

    body = f"{{ \
        \"variant\": {{ \
            \"sku\": \"{new_sku}\" \
        }} \
    }}"

    requests.put(f"https://{business_url}/admin/variants/{variant}.json", headers={"Content-Type": "application/json", "X-Shopify-Access-Token": access_token}, data=body)

def get_config():
    with open("config.json", "r") as f:
        return json.load(f)

def get_shopify_product_by_sku(sku: str):
    config = get_config()
    access_token = config["shopify_access_token"]
    business_url = config["business_url"]
    api_version = config["api_version"]

    body = f"{{ \
            productVariants(first: 10, query: \"sku:{sku}\") {{ \
                edges {{ \
                    node {{ \
                        id \
                        displayName \
                        barcode \
                    }} \
                }} \
            }} \
        }}"

    response = requests.post(f"https://{business_url}/admin/api/{api_version}/graphql.json", headers={"Content-Type": "application/graphql", "X-Shopify-Access-Token": access_token}, data=body)
    return response.json()["data"]["productVariants"]["edges"]

barcode_to_sku = {}
with open("./exported_data.csv", "r") as f:
    reader = csv.DictReader(f)

    for line in reader:
        barcode = line["Upc"]
        if barcode is None or not barcode.isdigit():
            continue

        while barcode.startswith("0"):
            barcode = barcode[1:]

        if len(barcode) > 12:
            continue

        barcode_to_sku[barcode] = (line["ID"], line["Description"])

with open("./possible_good_not_found.txt", "r") as f:
    skus = f.readlines()
    
    for s in skus:
        sku = s[:-1] if len(s) > 0 and s[-1] == "\n" else s 
        response = get_shopify_product_by_sku(sku)

        if len(response) == 0:
            print(f"Couldn't find shopify listing for {sku}")
            continue

        shopify_listing = response[0]["node"]
        barcode = shopify_listing["barcode"]
        if barcode is None:
            continue 

        while barcode.startswith("0"):
            barcode = barcode[1:]

        if barcode == "":
            continue
        
        if barcode not in barcode_to_sku:
            continue 

        real_sku = barcode_to_sku[barcode][0]
        should_replace = input(f"Replace {sku} with {real_sku} for {shopify_listing['displayName']} | {barcode_to_sku[barcode][1]}? ")

        if should_replace.lower() != "y":
            continue
        
        update_shopify_sku(shopify_listing["id"], real_sku)
