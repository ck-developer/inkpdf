// Long but finite loop; each iteration reads a template file, which gives the service
// a chance to interrupt the compilation.
#let iterations = sys.inputs.data.at("iterations", default: 200000)
#let total = 0
#for i in range(iterations) {
  total += read("payload.txt").len() + i
}
Done: #total
