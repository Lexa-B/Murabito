# exp-06 worldgen: reference readings

Every facts row of `viz/banana.html` at the defaults (commit `952747a`, 2026-09-30): seed 1, θ 12°, d +10, A₀ 250 m, H 0.65, stretch 2, E₀ 200 m, first octave 2, 8 octaves, 1.5 stripes per cell, σ_ref 3%, stacking p 1, normalise above 0.30, base for pivots true slope, lakes kept above 100 m, window the ri nearest 12 ri inland, (−6, −8, 14). Produced by `node viz/tools/check.js viz/banana.html`. The port is checked against these (see `docs/specs/2026-09-30-exp-06-worldgen-port-design.md`, §9). Render times are the page's and not to be matched.

```
-- facts
sea edges | 1  (SSE)
land edges | 3  (NNE, NNW, W)
shore edges (crossed) | 2  (E, SSW)
shore crossings | 2
no more than four of either | holds
world centre inside the banana | yes
-- facts2
spine height here, H(s) | 2455 m ≈ 16,200 layers
world centre: a across, s along | 0.67, 0.58
height at the origin | 269 m
edge ranges, m | E -85…4 · NNE 4…791 · NNW 791…2293 · W 646…2293 · SSW -31…646 · SSE -85…-31
slope just inland of the coast here | 21.5 m per ri: 100 m of height is 4.6 ri of coast
-- facts3
at the origin: coldest / mean / warmest month | 1.0 / 12.7 / 24.5 °C
at the origin: precipitation | 1724 mm
across the world's land ri | coldest month mean: -13.8 … 4.6 · annual mean: -0.8 … 15.1 · warmest month mean: 12.2 … 25.7 · precipitation: 1479 … 2885
-- facts4
ri above sea level, step 2 → step 4 | 1069 → 939 of 1,296 (82% → 72%)
height at the origin, step 2 → step 4 | 269 → 269 m
noise over the world's ri | -352 to 345 m, of a possible ±617
heavy side | inland, as seeded · mean inland +95 m, seaward -94 m
the arc's grain here | heading 56° from east · lattice 72 × 36 ri at octave 0
amplitude at a 12-ri wavelength | 122 m: on the plain at 21.5 m per ri that is 6 ri of coast
-- facts5
first octave | 2 · cells 9.00 ri · stripes 6.00 ri · E = 81 m ≈ 536 layers
ri above sea level, step 4 → 5a | 939 → 925 of 1,296 (72% → 71%)
at the origin | m 0.43, t -0.04 → +93 m: 269 → 362 m
erosion over the world's ri | -172 to +168 m · mean |Δ| on land 45 m
pivots at the first octave | 31 used · downhill headings 138° ± 28° (mod 180°): some disagreement, the stripes bend and break
window round ri (-6, -8, 14), 12 ri inland by default; click a ri on any step-5 map | mean 1097 → 1112 m · relief 300 → 439 m over 9,072 chō
-- facts5b
octaves | 2 (9.00 ri, E 81 m) · 3 (4.50 ri, E 52 m) · 4 (2.25 ri, E 33 m) · 5 (1.13 ri, E 21 m) · 6 (0.56 ri, E 13 m) · 7 (0.28 ri, E 9 m) · 8 (0.14 ri, E 5 m) · 9 (0.07 ri, E 3 m)
ri above sea level, step 4 → 5b | 939 → 928 of 1,296 (72% → 72%)
erosion over the world's ri | -179 to +202 m · mean |Δ| on land 47 m
octave 3 pivots turned by the gullies above | 30° on average over 109 nodes
octave 4 pivots turned by the gullies above | 48° on average over 335 nodes
octave 5 pivots turned by the gullies above | 53° on average over 1136 nodes
octave 6 pivots turned by the gullies above | 58° on average over 4151 nodes
octave 7 pivots turned by the gullies above | 61° on average over 6071 nodes
octave 8 pivots turned by the gullies above | 62° on average over 5194 nodes
octave 9 pivots turned by the gullies above | 64° on average over 3520 nodes
window round ri (-6, -8, 14) relief, 5a → 5b | 439 → 388 m over 9,072 chō
-- facts5c
ri above sea level, step 4 → 5c | 939 → 925 of 1,296 (72% → 71%)
erosion over the world's ri, 5b → 5c | mean |Δ| on land 47 → 72 m · range -211 to +216 m
room left for the last octave, on sloping land | 52% of its slope mask on average over 922 ri · the stacking halved it or more on 450 of them
window round ri (-6, -8, 14) relief, 5b → 5c | 388 → 516 m over 9,072 chō
-- facts5d
ri above sea level, step 4 → 5d | 939 → 914 of 1,296 (72% → 71%)
erosion over the world's ri, 5c → 5d | mean |Δ| on land 72 → 88 m · range -218 to +217 m
how much the pivots agreed, first octave, sloping land | blended pair length 0.83 on average over 911 ri · normalised on 876, left alone on 35
window round ri (-6, -8, 14) relief, 5c → 5d | 516 → 475 m over 9,072 chō
-- facts6
land share, step 2 → 4 → 5d, at d +10.0 | 82% → 72% → 71% · erosion costs 1.9 points
inset for 70% land after erosion | d ≈ +9.8 ri, where step 4 gives 72% · set the inset slider there
coast at ri scale | 96 coast ri · D ≈ 1.02 over 1–8 ri boxes · Kantō 1.1–1.15 · structure alone 1.35
coast at chō scale, window round ri (-6, -8, 14) | the coast does not cross the window · click a coastal ri
-- the strip button (STRIP=1)
the strip | 246 ri, the coast ri and their sea neighbours · 318,816 chō · 12,133 coast chō
boxes holding coast, by side in chō | 1 → 11,003 · 2 → 6,184 · 4 → 2,878 · 8 → 1,235 · 16 → 502 · 32 → 187 · 64 → 73
D by the boxes fitted | 1–16 chō: 1.12 · 2–32 chō: 1.26 · 4–64 chō: 1.33
-- factsGradient
at the origin | 61.7 m per ri, 1.6% · 0.9°, downhill heading 225° from east → face SSW
ri draining through each face | E 352 · NNE 158 · NNW 80 · W 116 · SSW 254 · SSE 336
steepest ri | 446 m per ri, 11.4% · 6.5° at ri (-6, -6, 12), 792 m
mean slope over land ri | 145.7 m per ri, 3.7% · 2.1°
ri at or past full arrow length | 476 of 1,296
-- factsRain
one ri | 13.36 km² · 1 mm = 13,355 m³
at the origin | 1837 mm → 24.5 million m³
wettest ri | 41.0 million m³ (3068 mm) at (-10, -13, 23)
driest ri | 15.3 million m³ (1148 mm) at (-12, 22, -10)
the world, a year | 30.51 km³ · on land 23.39 km³ · mean per ri 23.5 million m³
-- factsSettle
a year's rain in, and where it ends | 30514 M m³ = off the map 16853 + held in lakes 13661 · balances
off the map, by the world's edge crossed | E 2891 · NNE 472 · NNW 163 · W 57 · SSW 7619 · SSE 5651
pits found | 63 · 33 with fresh capacity above 0 m · 33 hold water over 60 ri
largest lake by water held | 1770.6 M m³ of 1997 capacity · 4 ri · floor -7 m (a cut-off sea basin) · surface at 55.1 m, spill at 62 m
largest flow through a land ri | 3728 M m³ a year through ri (-3,7,-4) into a sea ri by its SSE face
outlet faces used, all ri | E 412 · NNE 96 · NNW 23 · W 71 · SSW 280 · SSE 414
-- factsCarve
lakes deeper than 100 m stay | 33 pits breached in 1 round · 52 ri carved · deepest cut 144 m · 40549 M m³ of rock removed
a year's rain held in lakes, 7c → 7d | 13661 → 0 M m³ (45% → 0%) · off the map 16853 → 30514
lakes, 7c → 7d | 33 → 0
the largest river | 6338 M m³ a year through ri (-6,7,-1) into the sea by its SSE face
off the map, by the world's edge crossed | E 5479 · NNE 472 · NNW 163 · W 57 · SSW 15435 · SSE 8908
```

Averages over 50 seeds at the same defaults (`node viz/tools/seeds.js viz/banana.html 50`): land after step 4 75.6%, after 5d 73.0% (sd 3.8, 65–79); erosion's cost 2.6 points; coast D at ri scale 1.07 (0.87–1.24); window relief after 5d 562 m (sd 93). Over 20 seeds: rain held in lakes after 7c 51%, after 7d at 100 m 18%, lakes kept 2.8, ri carved 43.
