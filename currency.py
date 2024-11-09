import requests
import json
from datetime import datetime

rates = {}

start_year = 2000
current_year = start_year
current_month = 1

now = datetime.now()
total_months = 0

while (current_year < now.year) or (
    current_year == now.year and current_month <= now.month
):
    print(f"month: {current_year}-{current_month:02d}")

    total_months += 1
    # https://www.xe.com/currencytables/
    data = requests.get(
        f"https://www.xe.com/_next/data/lFsrCXsUT1R4egR02xO0Y/en/currencytables.json?from=USD&date={current_year}-{current_month:02d}-01"
    ).json()

    for currency in data["pageProps"]["historicRates"]:
        symbol = currency["currency"].lower()

        if symbol not in rates:
            rates[symbol] = []

        rates[symbol].append(currency["rate"])

    if current_month == 12:
        current_month = 1
        current_year += 1
    else:
        current_month += 1

for k in list(rates.keys()):
    if len(rates[k]) != total_months:
        rates.pop(k)
        print(f"removed {k}, didnt exist since year {start_year}")

with open("rates.json", "w") as file:
    json.dump(rates, file)
