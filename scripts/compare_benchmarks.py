import csv

def load_data(filepath):
    data = {}
    with open(filepath, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            data[row['graph']] = row
    return data

old_data = load_data("benchmark_120s.csv")
new_data = load_data("benchmark_union_standard.csv")

print("| Graph | Old Time (s) | New Time (s) | Time Diff (%) | Old Actions | New Actions | Action Diff (%) |")
print("|---|---|---|---|---|---|---|")

for graph, new_row in new_data.items():
    if graph not in old_data:
        continue
    old_row = old_data[graph]
    
    old_none_time = old_row['none_wall']
    new_none_time = new_row['none_wall']
    
    old_none_act = old_row['none_actions']
    new_none_act = new_row['none_actions']
    
    try:
        old_time_val = float(old_none_time)
        new_time_val = float(new_none_time)
        time_diff = f"{round(((new_time_val - old_time_val) / old_time_val) * 100, 1)}%" if old_time_val < 120 else "N/A"
    except:
        time_diff = "N/A"
        
    try:
        old_act_val = int(old_none_act)
        new_act_val = int(new_none_act)
        act_diff = f"{round(((new_act_val - old_act_val) / old_act_val) * 100, 1)}%"
    except:
        act_diff = "N/A"
        
    graph_name = graph.split("/")[-1]
    print(f"| {graph_name} | {old_none_time}s | {new_none_time}s | {time_diff} | {old_none_act} | {new_none_act} | {act_diff} |")
