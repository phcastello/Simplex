#pragma once
#include <vector>
#include <cmath>
#include <stdexcept>

class Matrix{
private:
    std::vector<std::vector<double>> data_;

public:
    // Construtores
    Matrix() = default;
    Matrix(std::size_t rows, std::size_t cols, double initialValue = 0.0);
    Matrix(const std::vector<std::vector<double>>& data);
    Matrix(std::vector<std::vector<double>>&& data);

    // Metodos auxiliares
    std::size_t rows() const;
    std::size_t cols() const;
    bool empty() const;
    bool isRectangular() const;
    bool isSquare() const;
    void push_back(std::vector<double> value);
    double& at(std::size_t i, std::size_t j);
    const double& at(std::size_t i, std::size_t j) const;
    
    // Metodos de cálculo
    // Calcula o determinante por laplace
    double determinant(Matrix B, int signal) const;
    Matrix inverse(Matrix B) const;
};
