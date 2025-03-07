# USAGE: benchstat -format scv ... | python3 plot_benchstat.py <benchmark>
# 
# To export to other formats just adjust the file_format variable.
#
# REQUIREMENTS:
# pip3 install matplotlib csv

import sys
import matplotlib.pyplot as plt
import matplotlib
import csv

file_format = "png" # "svg"

def parse_csv(csv_input):
    reader = list(csv.reader(csv_input.strip().split("\n")[4:]))
    
    interpreters = reader[0][1:]
    
    data = {}
    for (j, row) in enumerate(reader[2:]):
        benchmark = row[0]
        benchmark_data = {}
        for (i, cell) in enumerate(row[1:]):
            if i == 0 or (i-2) % 4 == 0: # time column
                interpreter = interpreters[i]
                benchmark_data[interpreter] = float(cell)
        data[benchmark] = benchmark_data
    
    return data

def generate_colors(num_colors):
    color_map = matplotlib.colormaps["tab20"]
    return [color_map(i / num_colors) for i in range(num_colors)]

def plot_benchmark(data):
    interpreters, times = list(zip(*data.items()))

    colors = generate_colors(len(interpreters))

    fig, ax = plt.subplots(figsize=(10, 6))
    bars = plt.bar(interpreters, times, color=colors[:len(interpreters)])
    for (bar, interpreter, time) in zip(bars, interpreters, times):
        height = ax.get_ylim()[1]
        if time > 1:
            label = f'{time:.3f}s'
        elif time > 1e-3:
            label = f'{time/1e-3:.3f}ms'
        elif time > 1e-6:
            label = f'{time/1e-6:.3f}µs'
        else:
            label = f'{time/1e-9:.3f}ns'
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            time + height * 0.01,
            label,
            ha='center',
            va='bottom',
            fontsize=10,
        )
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            height * 0.01,
            interpreter,
            ha='center',
            va='bottom',
            fontsize=10,
            rotation=90
        )

    plt.xlabel('Interpreter')
    plt.ylabel('Time in Seconds / Benchmark Run')
    plt.title(benchmark_name)

    plt.xticks([])
    plt.grid(True)
    plt.tight_layout()
    plt.show()

args = sys.argv
if len(args) != 2:
    print("USAGE: benchstat -format scv ... | python3 plot_benchstat.py <benchmark>")
    sys.exit()

benchmark_name = args[1]
benchstat_output = sys.stdin.read()
parsed_data = parse_csv(benchstat_output)
if benchmark_name in parsed_data:
    plot_benchmark(parsed_data[benchmark_name])
else:
    print(f"invalid benchmark name: {benchmark_name}")
