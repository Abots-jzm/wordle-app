import { useState, useRef } from "react";
import "./App.css";
import Cell, { CellState } from "./components/Cell";
import React from "react";

type Guess = {
	guess: string;
	correctness: CellState[];
};
function App() {
	const [history, setHistory] = useState([] as Guess[]);
	const [currentGuess, setCurrentGuess] = useState("");
	const [message, setmessage] = useState("");
	const [currentGuessState, setCurrentGuessState] = useState<CellState[]>(
		Array.from({ length: 5 }, () => "empty") as CellState[],
	);
	const inputRef = useRef<HTMLInputElement>(null);
	const formRef = useRef<HTMLFormElement>(null);

	function onSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		if (currentGuess.length !== 5) return setmessage("Guess must be 5 letters long.");
		if (currentGuessState.some((state) => state === "empty"))
			return setmessage("Please give all cells appriopriate colors by clicking them.");
		setmessage("");

		//Do rust stuff

		setHistory((prevHistory) => [...prevHistory, { guess: currentGuess, correctness: currentGuessState }]);
		setCurrentGuessState(Array.from({ length: 5 }, () => "empty") as CellState[]);
		setCurrentGuess("");
	}

	function resetButton() {
		setHistory([]);
		setCurrentGuess("");
		setCurrentGuessState(Array.from({ length: 5 }, () => "empty") as CellState[]);
		setmessage("");
		if (inputRef.current) inputRef.current.focus();
	}

	return (
		<div className="h-screen w-full bg-[#121213] p-10 text-white">
			<form ref={formRef} onSubmit={onSubmit} className="" onClick={() => inputRef.current?.focus()}>
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
							<Cell
								key={index}
								state="empty"
								changeOnClick
								cellIndex={index}
								setCurrentGuessState={setCurrentGuessState}
							>
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
				{message && <div className="text-red-500">{message}</div>}
			</form>
			<button className="mt-5 cursor-pointer rounded-lg bg-[#1f1f1f] p-2 px-4 text-white" onClick={resetButton}>
				Reset
			</button>
		</div>
	);
}

export default App;
