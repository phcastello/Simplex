#include "matrix.h"

//construtores
Matrix::Matrix(std::size_t rows, std::size_t cols, double initialValue)
    : data_(rows, std::vector<double>(cols, initialValue)) {}

Matrix::Matrix(const std::vector<std::vector<double>>& data) : data_(data) {
    if(!isSquare()){
        throw std::invalid_argument("Matriz invalida: linhas com tamanhos diferentes");
    }
}

Matrix::Matrix(std::vector<std::vector<double>>&& data) : data_(std::move(data)) {
    if(!isSquare()){
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

// Implementações dos métodos de cálculo

double Matrix::determinant(Matrix B, int signal) const{
    
}

Matrix Matrix::inverse(Matrix B) const{
    throw std::logic_error("Matrix::inverse() not implemented yet");
}
