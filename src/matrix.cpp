#include "matrix.h"

//construtores
Matrix::Matrix(std::size_t rows, std::size_t cols, double initialValue)
    : data_(rows, std::vector<double>(cols, initialValue)) {}

Matrix::Matrix(const std::vector<std::vector<double>>& data) : data_(data) {
    if(!isRectangular()){
        throw std::invalid_argument("Matriz invalida: linhas com tamanhos diferentes");
    }
}

Matrix::Matrix(std::vector<std::vector<double>>&& data) : data_(std::move(data)) {
    if(!isRectangular()){
        throw std::invalid_argument("Matriz invalida: linhas com tamanhos diferentes");
    }
}

// metodos auxiliares
std::size_t Matrix::rows() const{
    return data_.size();
}

std::size_t Matrix::cols() const{
    if(data_.empty()){
        return 0;
    }
    else{
        return data_[0].size();
    }
}

bool Matrix::empty() const{
    return data_.empty();
}

bool Matrix::isRectangular() const{
    if(data_.empty()){
        return true;
    }

    std::size_t expectedCols = data_[0].size();
    for(const std::vector<double>& row : data_){
        if(row.size() != expectedCols){
            return false;
        }
    }
    return true;
}

bool Matrix::isSquare() const{
    return !empty() && isRectangular() && rows() == cols();
}

void Matrix::push_back(std::vector<double> value){
    data_.push_back(value);
}

double& Matrix::at(std::size_t i, std::size_t j){
    return data_.at(i).at(j);
}

const double& Matrix::at(std::size_t i, std::size_t j) const{
    return data_.at(i).at(j);
}

// Helpers


// Gambiarra. Não use. Está aqui como recordação
Matrix Matrix::makeMinorMatrixGAMBIARRA(std::size_t linhaRemovida, std::size_t colunaRemovida) const{
    Matrix minorMatrix(this->rows()-1, this->cols()-1);
    std::vector<double> validValues;
    for(std::size_t i=0; i < this->rows(); i++){
        for(std::size_t j=0; j < this->cols(); j++){
            if(i != linhaRemovida and j != colunaRemovida){
                validValues.push_back(this->at(i,j));
            }
        }
    }
    size_t validValuesIndex = 0;
    if(!validValues.empty()) validValuesIndex = validValues.size();

    for(std::size_t i=minorMatrix.rows(); i-- > 0; ){
        for(std::size_t j=minorMatrix.cols(); j-- > 0; ){
            minorMatrix.at(i,j) = validValues.at(--validValuesIndex);
        }
    }

    return minorMatrix;
}

Matrix Matrix::makeMinorMatrix(std::size_t linhaRemovida, std::size_t colunaRemovida) const{
    if(linhaRemovida >= this->rows() or colunaRemovida >= this->cols()){
        throw std::out_of_range("linhaRemovida ou colunaRemovida estão fora dos limites da matriz");
    }
    Matrix minorMatrix(this->rows()-1, this->cols()-1);
    for(std::size_t i=0, mi = 0; i < this->rows(); i++){
        if(i != linhaRemovida){
            for(std::size_t j=0, mj = 0; j < this->cols(); j++){
                if(j != colunaRemovida){
                    minorMatrix.at(mi, mj) = this->at(i,j);
                    mj++;
                }
            }
            mi++;
        }
    }

    return minorMatrix;
}

// Implementações dos métodos de cálculo
double Matrix::determinant() const{

    if(!this->isSquare()){
        throw std::invalid_argument("A matriz precisa ser quadrada para o calculo do determinante.");
    }
    // Passo 1: Casos base

    // det de uma matriz 1x1 é seu proprio valor
    if(this->rows() == 1){
        return this->at(0,0);
    }

    // det de uma matriz 2x2 = A_11*A_22 - A_12*A_21
    if(this->rows() == 2){
        return (this->at(0,0)*this->at(1,1)) - (this->at(0,1)*this->at(1,0));
    }

    // Passo 2: chamar método auxiliar para criar a submatriz sem a linha e a coluna removida
    double det = 0;
    for(std::size_t j=0; j < this->cols(); j++){
        short signal;
        if(j % 2 == 0) signal = 1;
        else signal = -1;
        det += this->at(0,j) * signal * this->makeMinorMatrix(0, j).determinant();
    }
    return det;
    
}

Matrix Matrix::inverse() const{
    throw std::logic_error("Matrix::inverse() not implemented yet");
}
