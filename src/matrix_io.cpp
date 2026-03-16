#include "matrix_io.h"

#include <fstream>
#include <stdexcept>

#include "matrix_parser.h"

Matrix MatrixIO::ReadMatrix(const std::string& archivePath) const {
    std::ifstream matrixFile(archivePath);
    if (!matrixFile) {
        throw std::runtime_error("Erro ao abrir o arquivo" + archivePath + "\n");
    }

    MatrixParser parser;
    return parser.Parse(matrixFile);
}
