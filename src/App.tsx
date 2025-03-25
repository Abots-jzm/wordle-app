import { useState, useRef, useEffect } from "react";
import "./App.css";
import Cell, { CellState } from "./components/Cell";
import React from "react";
import { invoke } from "@tauri-apps/api/core";

// Match the struct on the Rust side for API correctness
type ApiCorrectness = "correct" | "misplaced" | "wrong";

// Match the GuessResult from Rust
type GuessResult = {
	word: string;
	score: number;
};

// Map our UI CellState to the API format
function mapCellStateToApiCorrectness(state: CellState): ApiCorrectness {
	switch (state) {
		case "green":
			return "correct";
		case "yellow":
			return "misplaced";
		case "gray":
			return "wrong";
		default:
			return "wrong";
	}
}

type Guess = {
	guess: string;
	correctness: CellState[];
};

function App() {
	const [history, setHistory] = useState([] as Guess[]);
	const [currentGuess, setCurrentGuess] = useState("");
	const [message, setMessage] = useState("");
	const [currentGuessState, setCurrentGuessState] = useState<CellState[]>(
		Array.from({ length: 5 }, () => "empty") as CellState[],
	);
	const [suggestions, setSuggestions] = useState<GuessResult[]>([]);
	const inputRef = useRef<HTMLInputElement>(null);
	const formRef = useRef<HTMLFormElement>(null);

	// Load the first guess on startup
	useEffect(() => {
		getInitialGuess();
	}, []);

	async function getInitialGuess() {
		try {
			const result = await invoke<GuessResult[]>("play", { history: [] });
			setSuggestions(result);
		} catch (error) {
			setMessage(`Error: ${error}`);
		}
	}

	async function onSubmit(e: React.FormEvent<HTMLFormElement>) {
		e.preventDefault();
		if (currentGuess.length !== 5) return setMessage("Guess must be 5 letters long.");
		if (currentGuessState.some((state) => state === "empty"))
			return setMessage("Please give all cells appropriate colors by clicking them.");

		// Create a new history entry
		const newHistoryEntry = {
			guess: currentGuess,
			correctness: currentGuessState,
		};

		// Update history
		const newHistory = [...history, newHistoryEntry];
		setHistory(newHistory);

		// Reset current guess state
		setCurrentGuessState(Array.from({ length: 5 }, () => "empty") as CellState[]);
		setCurrentGuess("");

		try {
			// Convert history to the format expected by Rust
			const apiHistory = newHistory.map((entry) => ({
				word: entry.guess,
				mask: entry.correctness.map(mapCellStateToApiCorrectness),
			}));

			// Call the Rust backend
			const result = await invoke<GuessResult[]>("play", { history: apiHistory });
			setSuggestions(result);
		} catch (error) {
			setMessage(`Error: ${error}`);
		}
	}

	async function resetButton() {
		setHistory([]);
		setCurrentGuess("");
		setCurrentGuessState(Array.from({ length: 5 }, () => "empty") as CellState[]);
		setSuggestions([]);
		setMessage("");

		try {
			// Reset the solver on the backend
			await invoke("reset");
			// Get new initial guess
			await getInitialGuess();
		} catch (error) {
			setMessage(`Error resetting: ${error}`);
		}

		if (inputRef.current) inputRef.current.focus();
	}

	return (
		<div className="h-screen w-full overflow-auto bg-[#121213] p-10 text-white">
			<div className="grid grid-cols-1 gap-8 md:grid-cols-2">
				<div>
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
								onChange={(e) => setCurrentGuess(e.target.value.toLowerCase())}
								value={currentGuess}
								ref={inputRef}
							/>
						</label>
						{message && <div className="mt-2 text-red-500">{message}</div>}
						<button
							type="button"
							className="mt-5 cursor-pointer rounded-lg bg-[#1f1f1f] p-2 px-4 text-white"
							onClick={resetButton}
						>
							Reset
						</button>
					</form>
				</div>

				<div className="rounded-lg bg-[#1a1a1a] p-6">
					<h2 className="mb-4 text-2xl font-bold">Suggestions</h2>
					{suggestions.length > 0 ? (
						<ul className="space-y-2">
							{suggestions.map((suggestion, index) => (
								<li
									key={index}
									className={`flex justify-between ${index === 0 ? "rounded bg-[#292929] p-2" : ""}`}
									onClick={() => {
										// When clicked, fill in the input with this suggestion
										setCurrentGuess(suggestion.word);
										inputRef.current?.focus();
									}}
								>
									<span className="cursor-pointer font-mono text-lg hover:underline">
										{suggestion.word.toUpperCase()}
									</span>
									<span className="text-gray-400">{(-suggestion.score).toFixed(2)}</span>
								</li>
							))}
						</ul>
					) : (
						<p>No suggestions available</p>
					)}
				</div>
			</div>
		</div>
	);
}

export default App;
