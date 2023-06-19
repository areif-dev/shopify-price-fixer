import sys
import csv
from argparse import ArgumentParser

SKU_START_INDEX = 19


def setup_args():
    parser = ArgumentParser(
        prog="sku_extractor",
        description="Fetch product skus from an ABC 2-14 (bill details) report.",
    )
    parser.add_argument(
        "-o", "--out", type=str, help="Path to optional file to print out skus"
    )
    parser.add_argument(
        "file_in", type=str, help="Path to the 2-14 report to extract skus from"
    )

    args = parser.parse_args()
    return args


def extract_skus_from_file(csv_reader: csv.DictReader) -> list[str]:
    skus = []
    for line in csv_reader:
        sku = line["Item"]

        if not sku.endswith(".") and not line["Cost"].strip() == "":
            skus.append(sku)

    return skus


def main():
    args = setup_args()

    with open(args.file_in) as file_214:
        csv_reader = csv.DictReader(file_214, delimiter="\t")
        skus = extract_skus_from_file(csv_reader)

    if args.out is not None and args.out != "":
        with open(args.out, "w") as f:
            [f.write(sku + "\n") for sku in skus]

        return

    [print(sku) for sku in skus]


if __name__ == "__main__":
    main()
