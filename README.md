# Shopify Price Fixer

## Mission

This software is intended to be used as a bridge between the proprietary ABC Accounting software and a Shopify website. 

## Usage

- On ABC, run a report from the 2-10 "Inventory Price Update" screen. This report takes a range of bill numbers as input and returns all items on those bills with their previous and up to date list prices. 
- Enter a range of bills that is moderate in size. For example, it is best to keep the total number of bills to no more than 50
- Xport the report to a file, and save it somewhere in the file system. You may save it anywhere and call it anything as long as you remember the path to where it was saved and what it is named
- Run the price-fixer binary. If you run the binary from the command line, you may supply the path to the report as a command line argument. If you do not supply a command line argument, the program will prompt you for the path to the report at runtime

## Examples

Suppose you have a report such as the following saved at C:\Users\user\Desktop\report.txt
```bash
# C:\Users\user\Desktop\report.txt
2-10 INVENTORY PRICE UPDATE            Saturday, September 3, 2022   67514     1
REIFSNYDER'S AG CENTER                                  Run:  9/ 3/2022  1:05 PM
                                                              UPDT  ORDER    BILL      OLD  NEW COST   %      OLD      NEW  %
ITEM # & DESCRIPTION                   UNIT MULT #CASES   QTY CODE  PRICE    PRICE     COST +FREIGHT   CHG    LIST     LIST MARKUP

# 67514 PURMII0 PURINA MILLS INC                 Order#
PURSTRATEGYH STRATEGY HEALTHY EDGE       EA                 40 D           20.9423  21.3423  20.9423   -2    24.25    24.25    16
PURSTRATEGY PURINA STRATEGY  (reg. Strat EA                 40 D           19.6619  20.1119  19.6619   -2    22.85    22.85    16
PURULTIUM ULTIUM HORSE FEED 11.7%-12.4%  EA                 40 D           25.8408  26.2908  25.8408   -2    28.85    28.85    12
PORCINE ACTIVE 25# PORCINE ACTIVE MAZURI                     3 D           14.6082  14.8382  14.6082   -2    18.49    18.49    27
PURAMPLIFY50 AMPLIFY SUPPLMNT 50# NUGGET EA                  5 D           44.5133  44.9633  44.5133   -1    52.99    52.99    19
- END -
```

### With command line args

```bash
.\price-fixer.exe C:\Users\user\Desktop\report.txt
```

### Without command line args

```bash
.\price-fixer.exe
Enter the path to the ABC 2-10 Report File: C:\Users\user\Desktop\report.txt
```
