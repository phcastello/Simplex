#include "matrix_parser.h"
#include <sstream>
#include <string>
#include <vector>

Matrix MatrixParser::Parse(std::istream& input) const {
    Matrix matrix;
    std::string stringLine;
    while (std::getline(input, stringLine)){
        std::stringstream ss(stringLine);

        std::vector<double> matrixLine;
        double value;

        while(ss >> value){
            matrixLine.push_back(value);
        }

        matrix.push_back(matrixLine);
    }

    return matrix;
}
