import { useState, useRef } from "react";
import "./App.css";
import Cell, { CellState } from "./components/Cell";
import React from "react";

type Guess = {
	guess: string;
	correctness: CellState[];
};

const TEMP_HISTORY: Guess[] = [
	{ guess: "HELLO", correctness: ["green", "green", "green", "green", "green"] },
	{ guess: "WORLD", correctness: ["yellow", "yellow", "yellow", "yellow", "yellow"] },
	{ guess: "APPLE", correctness: ["gray", "gray", "gray", "gray", "gray"] },
];

function App() {
	// const history = useState([] as Guess[]);
	const [history, setHistory] = useState(TEMP_HISTORY);
	const [currentGuess, setCurrentGuess] = useState("");
	const inputRef = useRef<HTMLInputElement>(null);
	const formRef = useRef<HTMLFormElement>(null);

	function onSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
	}

	function handleContainerClick() {
		// Focus the input when clicking anywhere in the container
		inputRef.current?.focus();
	}

	return (
		<div className="h-screen w-full bg-[#121213] p-10 text-white">
			<form ref={formRef} onSubmit={onSubmit} className="" onClick={handleContainerClick}>
				<label htmlFor="guess" className="grid h-max w-max grid-cols-5 gap-2">
					{history.map((guess, index) => (
						<React.Fragment key={`guess-row-${index}`}>
							{guess.guess
								.toUpperCase()
								.split("")
								.map((letter, index) => (
									<Cell key={index} state={guess.correctness[index]}>
										{letter}
									</Cell>
								))}
						</React.Fragment>
					))}
					{currentGuess
						.toUpperCase()
						.padEnd(5)
						.split("")
						.map((letter, index) => (
							<Cell key={index} state="empty">
								{letter}
							</Cell>
						))}
					{Array.from({ length: 5 - history.length }).map((_, rowIndex) => (
						<React.Fragment key={`empty-row-${rowIndex}`}>
							{Array.from({ length: 5 }).map((_, cellIndex) => (
								<Cell key={cellIndex} state="empty">
									{""}
								</Cell>
							))}
						</React.Fragment>
					))}
					<input
						id="guess"
						type="text"
						maxLength={5}
						className="h-0 w-0 opacity-0"
						onChange={(e) => setCurrentGuess(e.target.value)}
						value={currentGuess}
						ref={inputRef}
					/>
				</label>
			</form>
		</div>
	);
}

export default App;
