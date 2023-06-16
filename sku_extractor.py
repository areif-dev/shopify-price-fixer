import sys
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


def extract_skus_from_file(file_text: str) -> list[str]:
    lines = file_text.split("\n")
    skus = []
    for line in lines:

        # Lines that match these patterns are all header information, so they
        # can be skipped
        if (
            line.startswith("2-14")
            or line.startswith("REIFSNYDER'S")
            or line.startswith("Bill#")
            or line.strip() == ""
        ):
            continue

        sku_end_index = line.find("  ", SKU_START_INDEX)

        # There is no discernable end to the sku, so the line is most likely an
        # blank or missing information
        if sku_end_index < 0:
            continue

        skus.append(line[SKU_START_INDEX:sku_end_index])

    return skus


def main():
    args = setup_args()

    with open(args.file_in) as file_214:
        file_text = file_214.read()

    skus = extract_skus_from_file(file_text)

    if args.out is not None and args.out != "":
        with open(args.out, "w") as f:
            [f.write(sku + "\n") for sku in skus]

        return

    [print(sku) for sku in skus]


if __name__ == "__main__":
    main()
