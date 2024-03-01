lines = None
with open("1_28.txt", "r") as f:
    lines = f.readlines()

old_to_new_skus = {}
for line in lines:
    if line.startswith("1-28") or line.startswith("REIFSNYDER"):
        continue
    
    old_to_new_skus[line[5:23].strip()] = line[23:45].strip()
    old_to_new_skus[line[45:63].strip()] = line[63:].strip()

print(old_to_new_skus)

not_found_skus = []
with open("./not_found_skus.txt", "r") as f:
    not_found_skus = f.readlines()

with open("old_to_new_skus.txt", "w") as f:
    for old_sku in not_found_skus: 
        trimmed_old_sku = old_sku.strip()
        if trimmed_old_sku == "" or trimmed_old_sku not in old_to_new_skus:
            continue 

        f.write(f"{trimmed_old_sku}\t{old_to_new_skus[trimmed_old_sku]}\n")
