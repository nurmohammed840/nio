import argparse
import os
import subprocess
import re
import matplotlib.pyplot as plot

def run_wrk(url, duration, threads, connections):
    results = []
    for t in threads:
        for c in connections:
            cmd = ["wrk", f"-t{t}", f"-c{c}", f"-d{duration}", url]
            print(f"Running: {' '.join(cmd)}")
            try:
                output = subprocess.check_output(cmd, stderr=subprocess.STDOUT, text=True)
                # Extract Requests/sec using regex
                req_match = re.search(r"Requests/sec:\s+([\d.]+)", output)
                if req_match:
                    req_per_sec = float(req_match.group(1))
                    results.append((t, c, req_per_sec))
            except subprocess.CalledProcessError as e:
                print(f"Command failed: {e.output}")
                continue
                
    return results

def generate_combined_graph(nio_results, tokio_results, connections, prefix):
    # Ensure the results directory exists
    results_dir = "hyper-server/results"
    os.makedirs(results_dir, exist_ok=True)

    for c in connections:
        nio_data = [result for result in nio_results if result[1] == c]
        tokio_data = [result for result in tokio_results if result[1] == c]

        if nio_data and tokio_data:
            nio_data.sort()  # Sort by thread count
            tokio_data.sort()

            nio_threads, nio_reqs = zip(*[(t, r) for t, _, r in nio_data])
            tokio_threads, tokio_reqs = zip(*[(t, r) for t, _, r in tokio_data])

            plot.figure(figsize=(10, 6))
            plot.plot(nio_threads, nio_reqs, marker='o', label='nio', color='blue')
            plot.plot(tokio_threads, tokio_reqs, marker='o', label='tokio', color='red')

            plot.title(f"Performance Comparison for {c} Connections")
            plot.xlabel("wrk -t")
            plot.ylabel("Requests/sec")
            plot.grid(True)
            plot.xticks(range(min(nio_threads), max(nio_threads) + 1, 2))  # Set X-axis scale by 2
            plot.legend()

            file_path = os.path.join(results_dir, f"{prefix}_bench_{c}_connections.png")
            plot.savefig(file_path)
            print(f"Saved combined plot for {c} connections as '{file_path}'")
            # Uncomment below to show plots interactively
            # plot.show()

if __name__ == "__main__":
    # Command-line arguments
    parser = argparse.ArgumentParser(description="Run wrk benchmarks and generate graphs for nio and tokio servers.")
    parser.add_argument("--prefix", type=str, default="comparison", help="Prefix for the output file names")
    parser.add_argument("--duration", type=int, default=10, help="Duration of each wrk test (in seconds)")
    args = parser.parse_args()

    # Configuration
    NIO_URL = "http://127.0.0.1:4000"
    TOKIO_URL = "http://127.0.0.1:5000"
    THREADS = [2, 4, 6, 8, 10, 12, 14, 16, 18, 20]
    CONNECTIONS = [50, 100, 500, 1000]

    print("Running benchmarks for NIO server...")
    nio_results = run_wrk(NIO_URL, args.duration, THREADS, CONNECTIONS)

    print("Running benchmarks for Nio server...")
    tokio_results = run_wrk(TOKIO_URL, args.duration, THREADS, CONNECTIONS)

    print("Generating combined graphs...")
    generate_combined_graph(nio_results, tokio_results, CONNECTIONS, args.prefix)
