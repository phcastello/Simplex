#include <iostream>
#include <cmath>
#include <iomanip>
#include <chrono>
#include "include/umbrella.h"

#define nl std::cout << "\n";
#define readPath "data/read.txt"
#define writePath "data/write.txt"
// Porta de entrada do programa
// Deve ser quem chama as funções de cálculo de matriz inversa e determinante NxN

std::string formatDouble(double value, int casasDecimais = 20) {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(casasDecimais) << value;

    std::string s = oss.str();

    if (s.find('.') != std::string::npos) {
        while (!s.empty() && s.back() == '0') {
            s.pop_back();
        }

        if (!s.empty() && s.back() == '.') {
            s.pop_back();
        }
    }

    return s;
}

int main() {
    int option = -1;
    MatrixIO io;
    Matrix matrix;
    
    try{
        matrix = io.ReadMatrix(readPath);
    }
    catch(const std::exception& error){
        std::cerr << error.what();nl
        return 1;
    }

    if(!matrix.isSquare()){
        std::cout << "Atencao: matriz não quadrada.\n";
        std::cout << "Algumas funcionalidades nao funcionarao.\n";
    }
    
    bool loop = true;
    double detMatrix = 0.0;
    bool detWasCalculatedBefore = false;

    do{
        std::cout << "=================================\n";
        std::cout << "Escolha uma opcao abaixo: \n";
        std::cout << "0 - Sair\n";
        std::cout << "1 - Imprimir matriz\n";
        std::cout << "2 - Determinante NxN\n";
        std::cout << "3 - Inversa NxN\n";
        std::cout << "Opcao: ";
        std::cin >> option;nl


        switch (option){
        case 0:
            loop = false;
            break;

        case 1:
            std::cout << "Matriz a ser calculada:\n";
            for(size_t i=0; i < matrix.rows(); i++){
                for(size_t j=0; j < matrix.cols(); j++){
                    std::cout << matrix.at(i,j) << " ";
                }
                nl
            }
            nl
            io.WriteMatrix(writePath, matrix);
            break;

        case 2:
            std::cout << "Calculo do Determinante da matriz apresentada no arquivo de leitura:\n";
            if(!matrix.isSquare()){
                std::cout << "Nao e possivel calcular o determinante de uma matriz nao quadrada\n";
                break;
            }
            try{
                auto inicio = std::chrono::steady_clock::now();
                std::cout << "Determinante da matriz: " << formatDouble(matrix.determinant());nl
                auto fim = std::chrono::steady_clock::now();
                auto duracao_us = std::chrono::duration_cast<std::chrono::microseconds>(fim - inicio);
                auto duracao_s = std::chrono::duration<double>(fim - inicio);
                std::cout << "Tempo de execucao: " << duracao_us.count() << " microsegundos ou " << duracao_s.count() << " segundos\n";
                detWasCalculatedBefore = true;
                nl
            }
            catch(const std::exception& error){
                std::cerr << error.what();nl
            }
            break;
        
        case 3:
            std::cout << "Calculo da Inversa da matriz apresentada no arquivo de leitura:\n";
            if(!matrix.isSquare()){
                std::cout << "Nao e possivel calcular a inversa de uma matriz nao quadrada\n";
                break;
            }
            try{
                if(!detWasCalculatedBefore){
                    detMatrix = matrix.determinant();
                    detWasCalculatedBefore = true;
                }
                if(detMatrix == 0.0){
                    std::cout << "A inversa de uma matriz cujo determinante e 0 nao existe.\n";
                    break;
                }

                Matrix inverse = matrix.inverse();
                std::cout << "Matriz inversa:\n";
                for(size_t i=0; i < matrix.rows(); i++){
                    for(size_t j=0; j < matrix.cols(); j++){
                        std::cout << matrix.at(i,j) << " ";
                    }
                    nl
                }
                nl
                io.WriteMatrix(writePath, inverse);
            }
            catch(const std::exception& error){
                std::cerr << error.what();nl
            }
            break;

        default:
            std::cout << "Opcao invalida.\n";
            break;
        }
    }while(loop);

    return 0;
}
