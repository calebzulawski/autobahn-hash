#!/usr/bin/env python3

import cbor2
import plotly.express as px
import glob
from pathlib import Path
import pandas as pd

df = []
for file in glob.glob('target/criterion/data/**/benchmark.cbor', recursive=True):
    file = Path(file)
    with open(file, 'rb') as f:
        meta = cbor2.load(f)
    with open(file.parent / meta['latest_record'], 'rb') as f:
        data = cbor2.load(f)
    
    df.append({
        'type': meta['id']['group_id'],
        'crate': meta['id']['function_id'],
        'size': meta['id']['throughput']['Bytes'],
        'mean': data['estimates']['mean']['point_estimate'],
    })

df = pd.DataFrame(df)

for ty, df in df.groupby(df.type):
    df = df.sort_values("size")
    df['throughput'] = df['size'] / df['mean'] # B/ns is also GB/s
    max = df['throughput'].max()
    fig = px.line(df,
        x="size",
        y="throughput",
        color='crate',
        log_x=True,
        range_y=[0,df['throughput'].max()],
        line_shape='spline',
        labels={'throughput': "Throughput (GB/s)", 'size': "Input Size (bytes)"},
        title=f"Throughput for {ty} inputs",
    )
    fig.show()
