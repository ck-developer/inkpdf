// Boucle longue mais finie ; chaque itération lit un fichier du template, ce qui laisse au
// service l'occasion d'interrompre la compilation.
#let iterations = sys.inputs.data.at("iterations", default: 200000)
#let total = 0
#for i in range(iterations) {
  total += read("payload.txt").len() + i
}
Done: #total
