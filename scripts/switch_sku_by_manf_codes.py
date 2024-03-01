import csv 
import requests 
import json

def update_shopify_sku(variant: int, new_sku: str):
    config = get_config()
    access_token = config["shopify_access_token"]
    business_url = config["business_url"]
    api_version = config["api_version"]

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
                    }} \
                }} \
            }} \
        }}"

    response = requests.post(f"https://{business_url}/admin/api/{api_version}/graphql.json", headers={"Content-Type": "application/graphql", "X-Shopify-Access-Token": access_token}, data=body)
    return response.json()["data"]["productVariants"]["edges"]
    

def all_shopify_products():
    config = get_config()
    storefront_url = config["storefront_url"]
    
    all_products = {}
    page = 0 
    while True:
        response = requests.get(f"https://{storefront_url}/products.json?limit=250&page={page}")

        if "products" not in response.json() or len(response.json()["products"]) == 0:
            return all_products

        for product in response.json()["products"]:
            for variant in product["variants"]:
                all_products[variant["sku"]] = variant["id"]

        page += 1

all_active_products = all_shopify_products()
with open("exported_data.csv") as f:
    reader = csv.DictReader(f)

    extra_codes = {}
    for line in reader:
        if line["ID"].upper().startswith("SS") and len(line["ID"]) == 7:
            continue

        for i in range(1, 4):
            extra_code = line[f"ManufacturerAndCode{i}"]
            if extra_code is not None and not extra_code.startswith("Code & Vendor fields are full on") and not extra_code.startswith("PURINAMILL ITEM CODE"):
                extra_code = extra_code.split(" ")

                for i in range(len(extra_code)):
                    next_code = " ".join(extra_code[:i + 1])
                    if next_code.strip() == "":
                        continue

                    extra_codes[next_code] = line["ID"]

                    if next_code not in all_active_products:
                        continue 

                    if line["ID"] in all_active_products:
                        print(f"{next_code} cannot be replaced with {line['ID']} because {line['ID']} is already in use")
                        continue 

                    variant_id = all_active_products[next_code]
                    print(f"Variant: {variant_id}, real_sku: {line['ID']}, link: {next_code}")
                    update_shopify_sku(variant_id, line["ID"])
