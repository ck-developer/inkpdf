#import "@preview/cetz"
#import "@preview/cetz-plot": plot
#cetz.canvas({
  plot.plot(size: (4, 3), x-tick-step: 1, y-tick-step: 2, {
    plot.add(((0, 0), (1, 1), (2, 4), (3, 9)))
  })
})
