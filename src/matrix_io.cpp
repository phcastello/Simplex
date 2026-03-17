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

void MatrixIO::WriteMatrix(const std::string& archivePath, Matrix matrix) const{
    std::ofstream writeFile(archivePath);
    if(!writeFile){
        throw std::runtime_error("Erro ao abrir o arquivo" + archivePath + "\n");
    }
    
    for (std::size_t i = 0; i < matrix.rows(); ++i) {
        for (std::size_t j = 0; j < matrix.cols(); ++j) {
            writeFile << matrix.at(i, j) << " ";
        }
        writeFile << "\n";
    }
}