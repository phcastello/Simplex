#include "matrix.h"
#include <stdexcept>

bool Matrix::isSquare() const{
    return !this->empty() && this->size() == (*this)[0].size();
}

double Matrix::determinant() const{
    throw std::logic_error("Matrix::determinant() not implemented yet");
}

Matrix Matrix::inverse() const{
    throw std::logic_error("Matrix::inverse() not implemented yet");
}
